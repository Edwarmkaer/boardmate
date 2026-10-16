//! Transport adapters. Platform-specific code stays inside this crate.
mod ble;
mod framing;
mod serial;
mod websocket;

use async_trait::async_trait;
use boardmate_core::TransportKind;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub use ble::{scan_ble, BleEndpoint};
pub use serial::LineTransport;
pub use websocket::WebSocketTransport;

pub const IO_TIMEOUT: Duration = Duration::from_secs(6);

#[async_trait]
pub trait DeviceTransport: Send {
    async fn exchange(&mut self, request: &str) -> Result<String, String>;
    async fn disconnect(&mut self) -> Result<(), String> {
        Ok(())
    }
}

fn default_baud() -> u32 {
    115_200
}
fn default_channel() -> u8 {
    1
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionConfig {
    pub kind: TransportKind,
    pub address: String,
    #[serde(default = "default_baud")]
    pub baud_rate: u32,
    #[serde(default = "default_channel")]
    pub channel: u8,
}

#[derive(Serialize)]
pub struct TransportSupport {
    pub kind: TransportKind,
    pub available: bool,
    pub description: String,
}

pub fn supported_transports() -> Vec<TransportSupport> {
    vec![
        TransportSupport {
            kind: TransportKind::Serial,
            available: true,
            description: "USB CDC / USB-UART".into(),
        },
        TransportSupport {
            kind: TransportKind::Websocket,
            available: true,
            description: "Wi-Fi / WebSocket".into(),
        },
        TransportSupport {
            kind: TransportKind::Ble,
            available: true,
            description: "Bluetooth LE / UART GATT".into(),
        },
        TransportSupport {
            kind: TransportKind::Classic,
            available: cfg!(target_os = "linux"),
            description: "Bluetooth Classic / RFCOMM (Linux)".into(),
        },
    ]
}

pub fn serial_ports() -> Result<Vec<String>, String> {
    tokio_serial::available_ports()
        .map(|ports| ports.into_iter().map(|port| port.port_name).collect())
        .map_err(|error| error.to_string())
}

pub fn validate_request(request: &str) -> Result<(), String> {
    if request.len() + 1 > framing::MAX_FRAME_BYTES {
        return Err("Request frame too large".into());
    }
    if request.contains(['\r', '\n']) {
        return Err("Request must be one JSON line".into());
    }
    serde_json::from_str::<serde_json::Value>(request)
        .map_err(|_| "Request must be valid JSON".to_string())?;
    Ok(())
}

pub async fn connect(config: &ConnectionConfig) -> Result<Box<dyn DeviceTransport>, String> {
    if config.address.trim().is_empty() {
        return Err("Connection address is required".into());
    }
    // BLE manages its own deadlines so initialization can disconnect on timeout.
    if config.kind == TransportKind::Ble {
        return Ok(Box::new(ble::BleTransport::connect(&config.address).await?));
    }
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        match config.kind {
            TransportKind::Serial => Ok(Box::new(LineTransport::serial(
                &config.address,
                config.baud_rate,
            )?) as Box<dyn DeviceTransport>),
            TransportKind::Websocket => Ok(Box::new(
                WebSocketTransport::connect(&config.address).await?,
            ) as Box<dyn DeviceTransport>),
            TransportKind::Ble => Ok(Box::new(ble::BleTransport::connect(&config.address).await?)
                as Box<dyn DeviceTransport>),
            TransportKind::Classic => classic(config).await,
            TransportKind::Simulator => Err("Use the explicitly labeled simulator".into()),
        }
    })
    .await;
    result.map_err(|_| "Device connection timed out".to_string())?
}

#[cfg(target_os = "linux")]
async fn classic(config: &ConnectionConfig) -> Result<Box<dyn DeviceTransport>, String> {
    if !(1..=30).contains(&config.channel) {
        return Err("RFCOMM channel must be between 1 and 30".into());
    }
    let address = config
        .address
        .parse()
        .map_err(|_| "Invalid Bluetooth Classic address")?;
    let stream =
        bluer::rfcomm::Stream::connect(bluer::rfcomm::SocketAddr::new(address, config.channel))
            .await
            .map_err(|error| error.to_string())?;
    Ok(Box::new(LineTransport::new(stream)))
}

#[cfg(not(target_os = "linux"))]
async fn classic(_config: &ConnectionConfig) -> Result<Box<dyn DeviceTransport>, String> {
    Err("Bluetooth Classic adapter is currently implemented only on Linux".into())
}
