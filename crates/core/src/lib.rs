use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub id: String,
    pub device_id: String,
    pub action: String,
    pub arguments: Value,
}

pub fn validate_command(command: &Command, capabilities: &[String]) -> Result<(), String> {
    if command.id.trim().is_empty() || command.id.len() > 128 {
        return Err("Invalid request id".into());
    }
    if !capabilities.contains(&command.action) {
        return Err("Unsupported device capability".into());
    }
    if !command.arguments.is_object() {
        return Err("Invalid arguments".into());
    }
    match command.action.as_str() {
        "move_servo" => {
            let angle = command.arguments["angle"].as_u64();
            if !angle.is_some_and(|angle| angle <= 180) {
                return Err("Servo angle must be an integer between 0 and 180".into());
            }
        }
        "set_light" if !command.arguments["enabled"].is_boolean() => {
            return Err("Light enabled must be a boolean".into());
        }
        "read_temperature" | "set_light" => {}
        _ => return Err("Unknown device capability".into()),
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    Simulator,
    Serial,
    Websocket,
    Ble,
    Classic,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_c: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servo_angle: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub light_on: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub board: String,
    pub transport: TransportKind,
    pub simulated: bool,
    pub capabilities: Vec<String>,
    pub state: DeviceState,
}

impl Device {
    pub fn validate(&self, command: &Command) -> Result<(), String> {
        if command.device_id != self.id {
            return Err("Command addresses another device".into());
        }
        validate_command(command, &self.capabilities)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceReply {
    pub request_id: String,
    pub device_id: String,
    pub ok: bool,
    pub state: Option<DeviceState>,
    pub error: Option<String>,
}

#[async_trait::async_trait]
pub trait DeviceExecutor: Send + Sync {
    async fn devices(&self) -> Vec<Device>;
    async fn execute(&self, command: Command) -> Result<DeviceReply, String>;
}

pub struct Simulator {
    devices: BTreeMap<String, Device>,
    recent: VecDeque<(Command, DeviceReply)>,
}

impl Default for Simulator {
    fn default() -> Self {
        let devices: Vec<Device> = serde_json::from_value(json!([
            {"id":"demo-esp32","name":"Mesa de trabajo","board":"ESP32","transport":"simulator","simulated":true,"capabilities":["move_servo","set_light"],"state":{"servoAngle":90,"lightOn":false}},
            {"id":"demo-pico","name":"Ambiente","board":"Raspberry Pi Pico","transport":"simulator","simulated":true,"capabilities":["read_temperature"],"state":{"temperatureC":24.5}}
        ])).expect("built-in device descriptors are valid");
        Self {
            devices: devices
                .into_iter()
                .map(|device| (device.id.clone(), device))
                .collect(),
            recent: VecDeque::new(),
        }
    }
}

impl Simulator {
    pub fn devices(&self) -> Vec<Device> {
        self.devices.values().cloned().collect()
    }

    pub fn execute(&mut self, command: Command) -> Result<DeviceReply, String> {
        if let Some((previous, reply)) = self
            .recent
            .iter()
            .find(|(previous, _)| previous.id == command.id)
        {
            return if previous == &command {
                Ok(reply.clone())
            } else {
                Err("Reused request id with different command".into())
            };
        }
        let device = self
            .devices
            .get_mut(&command.device_id)
            .ok_or("Unknown device")?;
        device.validate(&command)?;
        match command.action.as_str() {
            "move_servo" => {
                device.state.servo_angle = command.arguments["angle"]
                    .as_u64()
                    .map(|angle| angle as u16)
            }
            "set_light" => device.state.light_on = command.arguments["enabled"].as_bool(),
            "read_temperature" => {}
            _ => return Err("Unsupported device capability".into()),
        }
        let reply = DeviceReply {
            request_id: command.id.clone(),
            device_id: device.id.clone(),
            ok: true,
            state: Some(device.state.clone()),
            error: None,
        };
        if self.recent.len() == 64 {
            self.recent.pop_front();
        }
        self.recent.push_back((command, reply.clone()));
        Ok(reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn servo(angle: Value) -> Command {
        Command {
            id: "1".into(),
            device_id: "bench".into(),
            action: "move_servo".into(),
            arguments: json!({"angle": angle}),
        }
    }

    #[test]
    fn accepts_valid_servo_command() {
        assert!(validate_command(&servo(json!(45)), &["move_servo".into()]).is_ok());
    }

    #[test]
    fn rejects_out_of_range_and_non_integer_angles() {
        for angle in [json!(-1), json!(181), json!(45.5), json!("45"), Value::Null] {
            assert!(validate_command(&servo(angle), &["move_servo".into()])
                .unwrap_err()
                .contains("angle"));
        }
    }

    #[test]
    fn simulator_changes_only_the_addressed_board() {
        let mut simulator = Simulator::default();
        let mut command = servo(json!(45));
        command.device_id = "demo-esp32".into();
        let reply = simulator.execute(command).unwrap();
        assert_eq!(reply.state.unwrap().servo_angle, Some(45));
        assert_eq!(simulator.devices()[1].state.temperature_c, Some(24.5));
    }

    #[test]
    fn invalid_command_does_not_change_device_state() {
        let mut simulator = Simulator::default();
        let mut command = servo(json!(400));
        command.device_id = "demo-esp32".into();
        assert!(simulator.execute(command).is_err());
        assert_eq!(simulator.devices()[0].state.servo_angle, Some(90));
    }

    #[test]
    fn reusing_request_id_for_different_action_is_rejected() {
        let mut simulator = Simulator::default();
        let command = Command {
            device_id: "demo-esp32".into(),
            ..servo(json!(45))
        };
        simulator.execute(command.clone()).unwrap();
        assert_eq!(
            simulator
                .execute(command.clone())
                .unwrap()
                .state
                .unwrap()
                .servo_angle,
            Some(45)
        );
        let changed = Command {
            arguments: json!({"angle": 90}),
            ..command
        };
        assert!(simulator
            .execute(changed)
            .unwrap_err()
            .contains("request id"));
    }

    #[test]
    fn rejects_unknown_devices_and_unsupported_capabilities() {
        let mut simulator = Simulator::default();
        let command = servo(json!(45));
        assert!(simulator
            .execute(command.clone())
            .unwrap_err()
            .contains("device"));
        assert!(simulator
            .execute(Command {
                device_id: "demo-pico".into(),
                ..command
            })
            .unwrap_err()
            .contains("capability"));
    }
}
