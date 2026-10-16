use crate::{framing::MAX_FRAME_BYTES, DeviceTransport};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
    MaybeTlsStream, WebSocketStream,
};

pub struct WebSocketTransport(WebSocketStream<MaybeTlsStream<TcpStream>>);

impl WebSocketTransport {
    pub async fn connect(address: &str) -> Result<Self, String> {
        if !address.starts_with("ws://") && !address.starts_with("wss://") {
            return Err("Wi-Fi address must start with ws:// or wss://".into());
        }
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_FRAME_BYTES))
            .max_frame_size(Some(MAX_FRAME_BYTES));
        let (stream, _) = connect_async_with_config(address, Some(config), true)
            .await
            .map_err(|error| error.to_string())?;
        Ok(Self(stream))
    }
}

#[async_trait]
impl DeviceTransport for WebSocketTransport {
    async fn exchange(&mut self, request: &str) -> Result<String, String> {
        crate::validate_request(request)?;
        self.0
            .send(Message::Text(request.into()))
            .await
            .map_err(|error| error.to_string())?;
        while let Some(message) = self.0.next().await {
            match message.map_err(|error| error.to_string())? {
                Message::Text(text) => return Ok(text.to_string()),
                Message::Binary(bytes) => {
                    return String::from_utf8(bytes.to_vec())
                        .map_err(|_| "Device frame is not UTF-8".into())
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        Err("Device connection closed".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn exchanges_a_request_with_a_real_local_websocket_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut stream = tokio_tungstenite::accept_async(socket).await.unwrap();
            assert_eq!(
                stream.next().await.unwrap().unwrap().into_text().unwrap(),
                "{\"action\":\"describe\"}"
            );
            stream
                .send(Message::Text("{\"ok\":true}".into()))
                .await
                .unwrap();
        });
        let mut transport = WebSocketTransport::connect(&format!("ws://{address}"))
            .await
            .unwrap();
        assert_eq!(
            transport
                .exchange("{\"action\":\"describe\"}")
                .await
                .unwrap(),
            "{\"ok\":true}"
        );
        server.await.unwrap();
    }
}
