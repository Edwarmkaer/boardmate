use crate::{Completion, LlmProvider, Message, ToolCall, ToolDefinition};
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct DemoProvider;

#[async_trait]
impl LlmProvider for DemoProvider {
    fn name(&self) -> &str {
        "demo"
    }
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Completion, String> {
        if matches!(messages.last(), Some(Message::Tool { .. })) {
            let results: Vec<String> = messages
                .iter()
                .filter_map(|message| {
                    let Message::Tool { content, .. } = message else {
                        return None;
                    };
                    let reply: Value = serde_json::from_str(content).ok()?;
                    if reply["ok"] != true {
                        return Some(format!(
                            "Error: {}",
                            reply["error"].as_str().unwrap_or("device failure")
                        ));
                    }
                    let state = &reply["state"];
                    Some(if let Some(value) = state["temperatureC"].as_f64() {
                        format!("Temperatura simulada: {value} °C.")
                    } else {
                        format!(
                            "Orden confirmada por {}. Servo: {}°. Luz: {}.",
                            reply["deviceId"].as_str().unwrap_or("dispositivo"),
                            state["servoAngle"],
                            if state["lightOn"] == true {
                                "encendida"
                            } else {
                                "apagada"
                            }
                        )
                    })
                })
                .collect();
            return Ok(Completion {
                text: Some(results.join(" ")),
                tool_calls: vec![],
            });
        }
        let input = messages
            .iter()
            .rev()
            .find_map(|message| match message {
                Message::User(text) => Some(text.to_lowercase()),
                _ => None,
            })
            .unwrap_or_default();
        let text = |content: &str| {
            Ok(Completion {
                text: Some(content.into()),
                tool_calls: vec![],
            })
        };
        if input
            .split_whitespace()
            .any(|word| matches!(word, "si" | "cuando"))
        {
            return text("El modo demo admite órdenes directas. Configura Groq para interpretar condiciones.");
        }
        let mut calls = Vec::new();
        for tool in tools {
            let Some(device_id) = tool
                .parameters
                .pointer("/properties/device_id/enum/0")
                .and_then(Value::as_str)
            else {
                continue;
            };
            let mut arguments = json!({"device_id":device_id});
            let selected = match tool.name.as_str() {
                "read_temperature" => input.contains("temperatura"),
                "move_servo"
                    if input.contains("servo")
                        || input.contains("gira")
                        || input.contains("mueve") =>
                {
                    let angle = input
                        .split(|character: char| {
                            !character.is_ascii_digit() && character != '-' && character != '.'
                        })
                        .find_map(|part| part.parse::<f64>().ok());
                    let Some(angle) = angle else {
                        return text("Indica un ángulo: por ejemplo, gira el servo a 45 grados.");
                    };
                    arguments["angle"] = json!(angle as i64);
                    if angle.fract() != 0.0 {
                        arguments["angle"] = json!(angle);
                    }
                    true
                }
                "set_light" if input.contains("enciende") || input.contains("apaga") => {
                    arguments["enabled"] = json!(!input.contains("apaga"));
                    true
                }
                _ => false,
            };
            if selected {
                calls.push(ToolCall {
                    id: format!("demo-{}", tool.name),
                    name: tool.name.clone(),
                    arguments,
                });
            }
        }
        if calls.is_empty() {
            return text("Prueba: ¿qué temperatura hay?, gira el servo a 45 grados o enciende la luz. Modo demostración; configura Groq para conversar con IA.");
        }
        Ok(Completion {
            text: None,
            tool_calls: calls,
        })
    }
}
