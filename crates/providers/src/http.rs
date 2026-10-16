use crate::{Completion, LlmProvider, Message, ToolCall, ToolDefinition};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::time::Duration;

pub struct HttpProvider {
    client: reqwest::Client,
    endpoint: String,
    model: String,
    api_key: String,
    name: String,
}

impl HttpProvider {
    pub fn new(
        name: String,
        base_url: &str,
        model: String,
        api_key: String,
    ) -> Result<Self, String> {
        let url = reqwest::Url::parse(base_url).map_err(|_| "Invalid provider URL")?;
        let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if url.scheme() != "https" && !(url.scheme() == "http" && local) {
            return Err("Provider URL must use HTTPS, except for localhost".into());
        }
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("Provider URL cannot contain credentials, query or fragment".into());
        }
        if model.trim().is_empty() || api_key.trim().is_empty() {
            return Err("Provider model and API key are required".into());
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "Could not initialize provider client")?;
        Ok(Self {
            client,
            endpoint: format!("{}/chat/completions", base_url.trim_end_matches('/')),
            model,
            api_key,
            name,
        })
    }
}

fn message_json(message: &Message) -> Value {
    match message {
        Message::System(content) => json!({"role":"system","content":content}),
        Message::User(content) => json!({"role":"user","content":content}),
        Message::Tool { id, content } => json!({"role":"tool","tool_call_id":id,"content":content}),
        Message::Assistant { text, tool_calls } => {
            json!({"role":"assistant","content":text,"tool_calls":tool_calls.iter().map(|call| json!({"id":call.id,"type":"function","function":{"name":call.name,"arguments":call.arguments.to_string()}})).collect::<Vec<_>>()})
        }
    }
}

#[async_trait]
impl LlmProvider for HttpProvider {
    fn name(&self) -> &str {
        &self.name
    }
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Completion, String> {
        let mut body = json!({"model":self.model,"messages":messages.iter().map(message_json).collect::<Vec<_>>(),"temperature":0.1,"max_completion_tokens":1024});
        if !tools.is_empty() {
            body["tools"] = json!(tools.iter().map(|tool| json!({"type":"function","function":{"name":tool.name,"description":tool.description,"parameters":tool.parameters}})).collect::<Vec<_>>());
            body["tool_choice"] = json!("auto");
        }
        let mut response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|_| "Provider connection failed or timed out")?;
        if !response.status().is_success() {
            return Err(format!(
                "Provider returned HTTP {}",
                response.status().as_u16()
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Provider response interrupted")?
        {
            if bytes.len() + chunk.len() > 256 * 1024 {
                return Err("Provider response too large".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|_| "Provider returned malformed JSON")?;
        let message = value
            .pointer("/choices/0/message")
            .ok_or("Provider response is missing a message")?;
        let calls = match message.get("tool_calls") {
            Some(Value::Array(calls)) => calls.as_slice(),
            Some(Value::Null) | None => &[],
            _ => return Err("Provider returned malformed tool calls".into()),
        };
        let tool_calls = calls
            .iter()
            .map(|call| {
                let id = call["id"].as_str().ok_or("Missing tool call id")?;
                let name = call["function"]["name"]
                    .as_str()
                    .ok_or("Missing tool name")?;
                let arguments = call["function"]["arguments"]
                    .as_str()
                    .ok_or("Missing tool arguments")?;
                Ok(ToolCall {
                    id: id.into(),
                    name: name.into(),
                    arguments: serde_json::from_str(arguments)
                        .map_err(|_| "Provider returned malformed tool arguments")?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Completion {
            text: message["content"].as_str().map(str::to_owned),
            tool_calls,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_remote_plaintext_urls_and_embedded_credentials() {
        for url in [
            "http://example.com/v1",
            "https://user:secret@example.com/v1",
            "https://example.com/v1?key=secret",
        ] {
            assert!(
                HttpProvider::new("custom".into(), url, "model".into(), "secret".into()).is_err()
            );
        }
    }

    #[tokio::test]
    async fn sends_tools_to_configured_endpoint_and_parses_model_tool_call() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 4096];
            loop {
                let count = socket.read(&mut chunk).await.unwrap();
                assert!(count > 0);
                request.extend_from_slice(&chunk[..count]);
                if let Some(end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    if request.len() >= end + 4 + length {
                        assert!(headers.starts_with("post /v1/chat/completions "));
                        assert!(headers.contains("authorization: bearer test-key"));
                        let body: Value =
                            serde_json::from_slice(&request[end + 4..end + 4 + length]).unwrap();
                        assert_eq!(body["model"], "test-model");
                        assert_eq!(body["tools"][0]["function"]["name"], "read_temperature");
                        break;
                    }
                }
            }
            let body = json!({"choices":[{"message":{"content":null,"tool_calls":[{"id":"call-1","function":{"name":"read_temperature","arguments":"{\"device_id\":\"demo-pico\"}"}}]}}]}).to_string();
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        });
        let provider = HttpProvider::new(
            "custom".into(),
            &format!("http://{address}/v1"),
            "test-model".into(),
            "test-key".into(),
        )
        .unwrap();
        let completion = provider
            .complete(
                &[Message::User("Temperatura".into())],
                &[ToolDefinition {
                    name: "read_temperature".into(),
                    description: "Read temperature".into(),
                    parameters: json!({"type":"object"}),
                }],
            )
            .await
            .unwrap();
        assert_eq!(completion.tool_calls[0].arguments["device_id"], "demo-pico");
        server.await.unwrap();
    }
}
