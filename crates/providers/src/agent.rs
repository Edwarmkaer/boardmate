use async_trait::async_trait;
use boardmate_core::{Command, Device, DeviceExecutor, DeviceReply};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Clone, Debug)]
pub enum Message {
    System(String),
    User(String),
    Assistant {
        text: Option<String>,
        tool_calls: Vec<ToolCall>,
    },
    Tool {
        id: String,
        content: String,
    },
}

#[derive(Clone, Debug)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Clone, Debug)]
pub struct Completion {
    pub text: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Completion, String>;
    fn name(&self) -> &str;
}

#[derive(Debug, Serialize)]
pub struct AgentReply {
    pub text: String,
    pub executions: Vec<DeviceReply>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn interrupted(error: String, executions: Vec<DeviceReply>) -> Result<AgentReply, String> {
    if executions.is_empty() {
        return Err(error);
    }
    let completed = executions.iter().filter(|reply| reply.ok).count();
    Ok(AgentReply {
        text: format!("Se confirmaron {completed} acciones. La solicitud se interrumpió: {error}"),
        executions,
        error: Some(error),
    })
}

pub fn device_tools(devices: &[Device]) -> Vec<ToolDefinition> {
    [("read_temperature", "Read the current temperature in Celsius"), ("move_servo", "Set an absolute servo angle"), ("set_light", "Set a light on or off")].into_iter().filter_map(|(name, description)| {
        let ids: Vec<&str> = devices.iter().filter(|device| device.capabilities.iter().any(|capability| capability == name)).map(|device| device.id.as_str()).collect();
        if ids.is_empty() { return None; }
        let mut parameters = json!({"type":"object", "properties":{"device_id":{"type":"string","enum":ids}}, "required":["device_id"], "additionalProperties":false});
        if name == "move_servo" {
            parameters["properties"]["angle"] = json!({"type":"integer","minimum":0,"maximum":180});
            parameters["required"].as_array_mut().unwrap().push(json!("angle"));
        } else if name == "set_light" {
            parameters["properties"]["enabled"] = json!({"type":"boolean"});
            parameters["required"].as_array_mut().unwrap().push(json!("enabled"));
        }
        Some(ToolDefinition { name: name.into(), description: description.into(), parameters })
    }).collect()
}

pub async fn run_agent(
    provider: &dyn LlmProvider,
    executor: &dyn DeviceExecutor,
    input: &str,
) -> Result<AgentReply, String> {
    if input.trim().is_empty() || input.len() > 4_000 {
        return Err("Message must contain 1–4000 bytes".into());
    }
    let devices = executor.devices().await;
    let tools = device_tools(&devices);
    let context = serde_json::to_string(&devices).map_err(|error| error.to_string())?;
    let mut messages = vec![Message::System(format!("You are boardmate, a precise hardware companion. Reply in Spanish. Use only advertised tools and device IDs. Ask when a device or parameter is ambiguous. Never invent measurements or claim an action succeeded without an ok tool result. Devices marked simulated are demonstrations. Connected devices: {context}")), Message::User(input.into())];
    let mut executions = Vec::new();
    let mut seen = HashSet::new();
    for _ in 0..6 {
        let completion = match tokio::time::timeout(
            Duration::from_secs(25),
            provider.complete(&messages, &tools),
        )
        .await
        {
            Ok(Ok(completion)) => completion,
            Ok(Err(error)) => return interrupted(error, executions),
            Err(_) => return interrupted("Provider response timed out".into(), executions),
        };
        if completion.tool_calls.is_empty() {
            let Some(text) = completion.text.filter(|text| !text.trim().is_empty()) else {
                return interrupted("Provider returned no response".into(), executions);
            };
            let failures: Vec<&str> = executions
                .iter()
                .filter_map(|reply: &DeviceReply| {
                    if reply.ok {
                        None
                    } else {
                        reply.error.as_deref()
                    }
                })
                .collect();
            let text = if failures.is_empty() {
                text
            } else {
                format!(
                    "No se completaron todas las acciones: {}",
                    failures.join("; ")
                )
            };
            return Ok(AgentReply {
                text,
                executions,
                error: None,
            });
        }
        if completion.tool_calls.len() > 4 {
            return interrupted("Tool call limit exceeded".into(), executions);
        }
        messages.push(Message::Assistant {
            text: completion.text,
            tool_calls: completion.tool_calls.clone(),
        });
        for call in completion.tool_calls {
            if !seen.insert(call.id.clone()) {
                return interrupted(
                    "Repeated tool call id; action was not retried".into(),
                    executions,
                );
            }
            let request_id = uuid::Uuid::new_v4().to_string();
            let device_id = call.arguments["device_id"]
                .as_str()
                .unwrap_or("")
                .to_owned();
            let mut arguments = call.arguments.clone();
            if let Some(object) = arguments.as_object_mut() {
                object.remove("device_id");
            }
            let command = Command {
                id: request_id.clone(),
                device_id: device_id.clone(),
                action: call.name,
                arguments,
            };
            let validation = devices
                .iter()
                .find(|device| device.id == device_id)
                .ok_or_else(|| "Unknown device".to_string())
                .and_then(|device| device.validate(&command));
            let result = match validation {
                Ok(()) => executor.execute(command).await,
                Err(error) => Err(error),
            };
            let reply = result.unwrap_or_else(|error| DeviceReply {
                request_id,
                device_id,
                ok: false,
                state: None,
                error: Some(error),
            });
            messages.push(Message::Tool {
                id: call.id,
                content: serde_json::to_string(&reply).map_err(|error| error.to_string())?,
            });
            executions.push(reply);
        }
    }
    interrupted(
        "Agent iteration limit reached; no more actions will run".into(),
        executions,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardmate_core::{Command, Device, Simulator};
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Mutex;

    struct Devices(Mutex<Simulator>);
    #[async_trait]
    impl DeviceExecutor for Devices {
        async fn devices(&self) -> Vec<Device> {
            self.0.lock().await.devices()
        }
        async fn execute(&self, command: Command) -> Result<DeviceReply, String> {
            self.0.lock().await.execute(command)
        }
    }

    struct Sequence {
        calls: AtomicUsize,
        angle: u64,
        repeat: bool,
    }
    #[async_trait]
    impl LlmProvider for Sequence {
        fn name(&self) -> &str {
            "test"
        }
        async fn complete(
            &self,
            messages: &[Message],
            _tools: &[ToolDefinition],
        ) -> Result<Completion, String> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call > 0 && !self.repeat {
                let Some(Message::Tool { content, .. }) = messages.last() else {
                    panic!("tool result missing from conversation")
                };
                let reply: DeviceReply = serde_json::from_str(content).unwrap();
                assert_eq!(reply.ok, self.angle <= 180);
                return Ok(Completion {
                    text: Some("Listo".into()),
                    tool_calls: vec![],
                });
            }
            Ok(Completion {
                text: None,
                tool_calls: vec![ToolCall {
                    id: format!("call-{call}"),
                    name: "move_servo".into(),
                    arguments: json!({"device_id":"demo-esp32", "angle":self.angle}),
                }],
            })
        }
    }

    #[tokio::test]
    async fn model_tool_call_changes_device_and_receives_real_result() {
        let provider = Sequence {
            calls: AtomicUsize::new(0),
            angle: 45,
            repeat: false,
        };
        let devices = Devices(Mutex::new(Simulator::default()));
        let reply = run_agent(&provider, &devices, "Gira el servo")
            .await
            .unwrap();
        assert_eq!(
            reply.executions[0].state.as_ref().unwrap().servo_angle,
            Some(45)
        );
        assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn invalid_model_arguments_do_not_change_actuator_state() {
        let provider = Sequence {
            calls: AtomicUsize::new(0),
            angle: 400,
            repeat: false,
        };
        let devices = Devices(Mutex::new(Simulator::default()));
        let reply = run_agent(&provider, &devices, "Gira el servo")
            .await
            .unwrap();
        assert!(!reply.executions[0].ok);
        assert_eq!(devices.devices().await[0].state.servo_angle, Some(90));
    }

    #[tokio::test]
    async fn agent_stops_a_model_that_keeps_requesting_tools() {
        let provider = Sequence {
            calls: AtomicUsize::new(0),
            angle: 45,
            repeat: true,
        };
        let devices = Devices(Mutex::new(Simulator::default()));
        let reply = run_agent(&provider, &devices, "Gira").await.unwrap();
        assert!(reply.error.unwrap().contains("limit"));
        assert!(provider.calls.load(Ordering::SeqCst) <= 6);
    }

    struct FailingFollowup(AtomicUsize);

    #[async_trait]
    impl LlmProvider for FailingFollowup {
        fn name(&self) -> &str {
            "failing-test"
        }
        async fn complete(
            &self,
            _messages: &[Message],
            _tools: &[ToolDefinition],
        ) -> Result<Completion, String> {
            if self.0.fetch_add(1, Ordering::SeqCst) > 0 {
                return Err("Provider unavailable after action".into());
            }
            Ok(Completion {
                text: None,
                tool_calls: vec![ToolCall {
                    id: "move".into(),
                    name: "move_servo".into(),
                    arguments: json!({"device_id":"demo-esp32","angle":45}),
                }],
            })
        }
    }

    #[tokio::test]
    async fn preserves_completed_actions_when_provider_followup_fails() {
        let provider = FailingFollowup(AtomicUsize::new(0));
        let devices = Devices(Mutex::new(Simulator::default()));
        let reply = run_agent(&provider, &devices, "Gira el servo")
            .await
            .unwrap();
        assert_eq!(reply.executions.len(), 1);
        assert_eq!(
            reply.executions[0].state.as_ref().unwrap().servo_angle,
            Some(45)
        );
        assert!(reply.text.contains("Provider unavailable"));
    }
}
