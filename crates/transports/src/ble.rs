use crate::{framing::MAX_FRAME_BYTES, DeviceTransport};
use async_trait::async_trait;
use btleplug::{
    api::{
        Central, CharPropFlags, Characteristic, Manager as _, Peripheral as _, ScanFilter,
        ValueNotification, WriteType,
    },
    platform::{Manager, Peripheral},
};
use futures_util::{Stream, StreamExt};
use serde::Serialize;
use std::{pin::Pin, time::Duration};
use uuid::Uuid;

const SERVICE: &str = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
const WRITE: &str = "6e400002-b5a3-f393-e0a9-e50e24dcca9e";
const NOTIFY: &str = "6e400003-b5a3-f393-e0a9-e50e24dcca9e";

async fn bounded<T>(
    future: impl std::future::Future<Output = Result<T, btleplug::Error>>,
) -> Result<T, String> {
    tokio::time::timeout(Duration::from_secs(3), future)
        .await
        .map_err(|_| "Bluetooth operation timed out".to_string())?
        .map_err(|error| error.to_string())
}

#[derive(Serialize)]
pub struct BleEndpoint {
    pub address: String,
    pub name: String,
}

async fn peripherals() -> Result<Vec<Peripheral>, String> {
    let manager = bounded(Manager::new()).await?;
    let adapter = bounded(manager.adapters())
        .await?
        .into_iter()
        .next()
        .ok_or("No Bluetooth adapter found")?;
    let started = bounded(adapter.start_scan(ScanFilter {
        services: vec![Uuid::parse_str(SERVICE).unwrap()],
    }))
    .await;
    if let Err(error) = started {
        let _ = bounded(adapter.stop_scan()).await;
        return Err(error);
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    let result = bounded(adapter.peripherals()).await;
    let stopped = bounded(adapter.stop_scan()).await;
    stopped?;
    result
}

pub async fn scan_ble() -> Result<Vec<BleEndpoint>, String> {
    let mut endpoints = Vec::new();
    for peripheral in peripherals().await?.into_iter().take(32) {
        let properties = bounded(peripheral.properties()).await?;
        let name = properties
            .and_then(|properties| properties.local_name)
            .unwrap_or_else(|| "BLE device".into());
        endpoints.push(BleEndpoint {
            address: peripheral.id().to_string(),
            name,
        });
    }
    Ok(endpoints)
}

pub struct BleTransport {
    peripheral: Peripheral,
    write: Characteristic,
    notify_uuid: Uuid,
    notifications: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    pending: Vec<u8>,
}

impl BleTransport {
    pub async fn connect(address: &str) -> Result<Self, String> {
        let peripheral = peripherals()
            .await?
            .into_iter()
            .find(|peripheral| peripheral.id().to_string() == address)
            .ok_or("BLE device not found; scan and use its exact address")?;
        let result = tokio::time::timeout(Duration::from_secs(8), async {
            peripheral
                .connect()
                .await
                .map_err(|error| error.to_string())?;
            Self::initialize(peripheral.clone()).await
        })
        .await
        .map_err(|_| "BLE connection timed out".to_string())
        .and_then(|result| result);
        if result.is_err() {
            let _ = bounded(peripheral.disconnect()).await;
        }
        result
    }

    async fn initialize(peripheral: Peripheral) -> Result<Self, String> {
        peripheral
            .discover_services()
            .await
            .map_err(|error| error.to_string())?;
        let characteristics = peripheral.characteristics();
        let write_uuid = Uuid::parse_str(WRITE).unwrap();
        let notify_uuid = Uuid::parse_str(NOTIFY).unwrap();
        let write = characteristics
            .iter()
            .find(|characteristic| characteristic.uuid == write_uuid)
            .cloned()
            .ok_or("BLE board does not expose the required UART write characteristic")?;
        let notify = characteristics
            .iter()
            .find(|characteristic| characteristic.uuid == notify_uuid)
            .ok_or("BLE board does not expose the required UART notify characteristic")?;
        let notifications = peripheral
            .notifications()
            .await
            .map_err(|error| error.to_string())?;
        peripheral
            .subscribe(notify)
            .await
            .map_err(|error| error.to_string())?;
        Ok(Self {
            peripheral,
            write,
            notify_uuid,
            notifications,
            pending: Vec::new(),
        })
    }
}

#[async_trait]
impl DeviceTransport for BleTransport {
    async fn exchange(&mut self, request: &str) -> Result<String, String> {
        crate::validate_request(request)?;
        let write_type = if self.write.properties.contains(CharPropFlags::WRITE) {
            WriteType::WithResponse
        } else {
            WriteType::WithoutResponse
        };
        for chunk in format!("{request}\n").as_bytes().chunks(20) {
            self.peripheral
                .write(&self.write, chunk, write_type)
                .await
                .map_err(|error| error.to_string())?;
        }
        loop {
            if let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
                let frame: Vec<u8> = self.pending.drain(..=end).collect();
                return String::from_utf8(frame)
                    .map(|text| text.trim_end_matches(['\r', '\n']).into())
                    .map_err(|_| "Device frame is not UTF-8".into());
            }
            let notification = self
                .notifications
                .next()
                .await
                .ok_or("BLE connection closed")?;
            if notification.uuid != self.notify_uuid {
                continue;
            }
            if self.pending.len() + notification.value.len() > MAX_FRAME_BYTES {
                return Err("BLE frame too large".into());
            }
            self.pending.extend_from_slice(&notification.value);
        }
    }
    async fn disconnect(&mut self) -> Result<(), String> {
        self.peripheral
            .disconnect()
            .await
            .map_err(|error| error.to_string())
    }
}
