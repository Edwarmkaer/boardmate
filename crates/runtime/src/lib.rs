use boardmate_core::{Command, Device, DeviceExecutor, DeviceReply, Simulator};
use boardmate_providers::{run_agent, AgentReply, DemoProvider, HttpProvider, LlmProvider};
use boardmate_transports::{ConnectionConfig, DeviceTransport, IO_TIMEOUT};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{Mutex, RwLock, Semaphore};

struct ConnectedDevice {
    device: Mutex<Device>,
    transport: Mutex<Box<dyn DeviceTransport>>,
    endpoint: String,
}

pub struct Runtime {
    provider: Arc<dyn LlmProvider>,
    simulator: Mutex<Simulator>,
    connected: RwLock<BTreeMap<String, Arc<ConnectedDevice>>>,
    connection_gate: Semaphore,
    agent_gate: Semaphore,
}

#[derive(Serialize)]
pub struct RuntimeInfo {
    pub platform: &'static str,
    pub provider: String,
    pub native: bool,
    pub transports: Vec<boardmate_transports::TransportSupport>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Description {
    request_id: String,
    device: Device,
}

impl Runtime {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            provider,
            simulator: Mutex::new(Simulator::default()),
            connected: RwLock::new(BTreeMap::new()),
            connection_gate: Semaphore::new(1),
            agent_gate: Semaphore::new(1),
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let name = std::env::var("BOARDMATE_PROVIDER").unwrap_or_else(|_| "demo".into());
        if name == "demo" {
            return Ok(Self::new(Arc::new(DemoProvider)));
        }
        if name != "groq" && name != "compatible" {
            return Err("BOARDMATE_PROVIDER must be demo, groq or compatible".into());
        }
        let base = std::env::var("BOARDMATE_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.groq.com/openai/v1".into());
        let model =
            std::env::var("BOARDMATE_MODEL").unwrap_or_else(|_| "llama-3.3-70b-versatile".into());
        let key = std::env::var("BOARDMATE_API_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty())
            .or_else(|| {
                std::env::var("GROQ_API_KEY")
                    .ok()
                    .filter(|key| !key.trim().is_empty())
            })
            .ok_or("Configure GROQ_API_KEY or BOARDMATE_API_KEY in the native environment")?;
        Ok(Self::new(Arc::new(HttpProvider::new(
            name, &base, model, key,
        )?)))
    }

    pub fn info(&self) -> RuntimeInfo {
        RuntimeInfo {
            platform: std::env::consts::OS,
            provider: self.provider.name().into(),
            native: true,
            transports: boardmate_transports::supported_transports(),
        }
    }

    pub async fn chat(&self, message: &str) -> Result<AgentReply, String> {
        let _permit = self
            .agent_gate
            .try_acquire()
            .map_err(|_| "An assistant request is already running")?;
        run_agent(self.provider.as_ref(), self, message).await
    }

    pub async fn connect(&self, config: ConnectionConfig) -> Result<Device, String> {
        let _permit = self
            .connection_gate
            .try_acquire()
            .map_err(|_| "Another connection is being configured")?;
        let endpoint = format!("{:?}:{}", config.kind, config.address);
        {
            let connected = self.connected.read().await;
            if connected.len() >= 16 {
                return Err("Connected device limit reached".into());
            }
            if connected.values().any(|device| device.endpoint == endpoint) {
                return Err("Device endpoint is already connected".into());
            }
        }
        let mut transport = boardmate_transports::connect(&config).await?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = json!({"id":request_id,"action":"describe"}).to_string();
        let result = async {
            let frame = tokio::time::timeout(IO_TIMEOUT, transport.exchange(&request))
                .await
                .map_err(|_| "Device description timed out")??;
            let mut description: Description = serde_json::from_str(&frame)
                .map_err(|_| "Device returned an invalid description")?;
            if description.request_id != request_id {
                return Err("Device description request id mismatch".to_string());
            }
            let device = &mut description.device;
            if device.id.is_empty()
                || device.id.len() > 64
                || device.id.starts_with("demo-")
                || device.name.trim().is_empty()
                || device.name.len() > 80
                || device.board.len() > 80
            {
                return Err("Invalid or reserved device identity".into());
            }
            if device.capabilities.is_empty()
                || device.capabilities.len() > 3
                || device.capabilities.iter().any(|name| {
                    !matches!(
                        name.as_str(),
                        "read_temperature" | "move_servo" | "set_light"
                    )
                })
            {
                return Err("Device advertised unsupported capabilities".into());
            }
            if self.connected.read().await.contains_key(&device.id) {
                return Err("Device id is already connected".into());
            }
            device.transport = config.kind;
            device.simulated = false;
            Ok(description.device)
        }
        .await;
        match result {
            Ok(device) => {
                self.connected.write().await.insert(
                    device.id.clone(),
                    Arc::new(ConnectedDevice {
                        device: Mutex::new(device.clone()),
                        transport: Mutex::new(transport),
                        endpoint,
                    }),
                );
                Ok(device)
            }
            Err(error) => {
                let _ = tokio::time::timeout(IO_TIMEOUT, transport.disconnect()).await;
                Err(error)
            }
        }
    }

    pub async fn disconnect(&self, device_id: &str) -> Result<(), String> {
        let connected = self
            .connected
            .read()
            .await
            .get(device_id)
            .cloned()
            .ok_or("Unknown connected device")?;
        let mut transport = connected
            .transport
            .try_lock()
            .map_err(|_| "Device is busy; wait for the current action")?;
        self.connected.write().await.remove(device_id);
        tokio::time::timeout(IO_TIMEOUT, transport.disconnect())
            .await
            .map_err(|_| "Device disconnect timed out")?
    }
}

#[async_trait::async_trait]
impl DeviceExecutor for Runtime {
    async fn devices(&self) -> Vec<Device> {
        let mut devices = self.simulator.lock().await.devices();
        let connected: Vec<_> = self.connected.read().await.values().cloned().collect();
        for connection in connected {
            devices.push(connection.device.lock().await.clone());
        }
        devices
    }

    async fn execute(&self, command: Command) -> Result<DeviceReply, String> {
        if command.device_id.starts_with("demo-") {
            return self.simulator.lock().await.execute(command);
        }
        let connected = self
            .connected
            .read()
            .await
            .get(&command.device_id)
            .cloned()
            .ok_or("Unknown connected device")?;
        connected.device.lock().await.validate(&command)?;
        let mut transport = connected
            .transport
            .try_lock()
            .map_err(|_| "Device is busy; action was not queued or retried")?;
        let request = serde_json::to_string(&command).map_err(|error| error.to_string())?;
        let result = async {
            let frame = tokio::time::timeout(IO_TIMEOUT, transport.exchange(&request))
                .await
                .map_err(|_| "Device reply timed out; action was not retried")??;
            let reply: DeviceReply =
                serde_json::from_str(&frame).map_err(|_| "Device returned malformed reply")?;
            if reply.request_id != command.id || reply.device_id != command.device_id {
                return Err("Device reply identity mismatch".to_string());
            }
            if let Some(state) = &reply.state {
                let mut device = connected.device.lock().await;
                if state.temperature_c.is_some() {
                    device.state.temperature_c = state.temperature_c;
                }
                if state.servo_angle.is_some() {
                    device.state.servo_angle = state.servo_angle;
                }
                if state.light_on.is_some() {
                    device.state.light_on = state.light_on;
                }
            }
            Ok(reply)
        }
        .await;
        if result.is_err() {
            self.connected.write().await.remove(&command.device_id);
            let _ = tokio::time::timeout(IO_TIMEOUT, transport.disconnect()).await;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardmate_core::TransportKind;
    use boardmate_providers::DemoProvider;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};

    #[tokio::test]
    async fn connects_describes_and_controls_a_board_over_a_real_socket() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
            let describe: Value =
                serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap())
                    .unwrap();
            assert_eq!(describe["action"], "describe");
            let descriptor = json!({"requestId":describe["id"], "device":{"id":"test-board","name":"Test board","board":"ESP32","transport":"serial","simulated":true,"capabilities":["set_light"],"state":{"lightOn":false}}});
            socket
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    descriptor.to_string().into(),
                ))
                .await
                .unwrap();
            let command: Value =
                serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap())
                    .unwrap();
            assert_eq!(command["action"], "set_light");
            let reply = json!({"requestId":command["id"],"deviceId":"test-board","ok":true,"state":{"lightOn":true},"error":null});
            socket
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    reply.to_string().into(),
                ))
                .await
                .unwrap();
        });
        let runtime = Runtime::new(Arc::new(DemoProvider));
        let board = runtime
            .connect(ConnectionConfig {
                kind: TransportKind::Websocket,
                address: format!("ws://{address}"),
                baud_rate: 115200,
                channel: 1,
            })
            .await
            .unwrap();
        assert!(
            !board.simulated,
            "simulation flags must be controlled by the host"
        );
        assert_eq!(board.transport, TransportKind::Websocket);
        let invalid = Command {
            id: "bad".into(),
            device_id: board.id.clone(),
            action: "move_servo".into(),
            arguments: json!({"angle":45}),
        };
        assert!(runtime
            .execute(invalid)
            .await
            .unwrap_err()
            .contains("capability"));
        let reply = runtime
            .execute(Command {
                id: "light-1".into(),
                device_id: board.id,
                action: "set_light".into(),
                arguments: json!({"enabled":true}),
            })
            .await
            .unwrap();
        assert_eq!(reply.state.unwrap().light_on, Some(true));
        assert_eq!(runtime.devices().await.len(), 3);
        server.await.unwrap();
    }
}
