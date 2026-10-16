use crate::{framing::read_frame, DeviceTransport};
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio_serial::SerialPortBuilderExt;

pub trait AsyncIo: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncIo for T {}

pub struct LineTransport {
    stream: BufReader<Box<dyn AsyncIo>>,
}

impl LineTransport {
    pub fn new(stream: impl AsyncIo + 'static) -> Self {
        Self {
            stream: BufReader::new(Box::new(stream)),
        }
    }

    pub fn serial(path: &str, baud_rate: u32) -> Result<Self, String> {
        if baud_rate == 0 || baud_rate > 4_000_000 {
            return Err("Invalid baud rate".into());
        }
        let port = tokio_serial::new(path, baud_rate)
            .open_native_async()
            .map_err(|error| error.to_string())?;
        Ok(Self::new(port))
    }
}

#[async_trait]
impl DeviceTransport for LineTransport {
    async fn exchange(&mut self, request: &str) -> Result<String, String> {
        crate::validate_request(request)?;
        self.stream
            .get_mut()
            .write_all(format!("{request}\n").as_bytes())
            .await
            .map_err(|error| error.to_string())?;
        self.stream
            .get_mut()
            .flush()
            .await
            .map_err(|error| error.to_string())?;
        read_frame(&mut self.stream).await
    }
}
