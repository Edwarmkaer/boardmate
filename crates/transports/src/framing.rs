use tokio::io::{AsyncBufRead, AsyncBufReadExt};

pub const MAX_FRAME_BYTES: usize = 64 * 1024;

pub async fn read_frame<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<String, String> {
    let mut frame = Vec::new();
    loop {
        let available = reader.fill_buf().await.map_err(|error| error.to_string())?;
        if available.is_empty() {
            return Err("Device connection closed before a complete frame".into());
        }
        let end = available.iter().position(|byte| *byte == b'\n');
        let count = end.map_or(available.len(), |position| position + 1);
        if frame.len() + count > MAX_FRAME_BYTES {
            return Err("Device frame too large".into());
        }
        frame.extend_from_slice(&available[..count]);
        reader.consume(count);
        if end.is_some() {
            let text = String::from_utf8(frame).map_err(|_| "Device frame is not UTF-8")?;
            return Ok(text.trim_end_matches(['\r', '\n']).into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncWriteExt, BufReader};

    #[tokio::test]
    async fn reads_a_json_frame_split_across_writes() {
        let (mut writer, reader) = tokio::io::duplex(128);
        writer.write_all(b"{\"ok\":").await.unwrap();
        writer.write_all(b"true}\r\n").await.unwrap();
        let frame = read_frame(&mut BufReader::new(reader)).await.unwrap();
        assert_eq!(frame, "{\"ok\":true}");
    }

    #[tokio::test]
    async fn rejects_oversized_frames_without_unbounded_allocation() {
        let input = vec![b'x'; MAX_FRAME_BYTES + 1];
        let error = read_frame(&mut BufReader::new(input.as_slice()))
            .await
            .unwrap_err();
        assert!(error.contains("large"));
    }

    #[tokio::test]
    async fn reports_disconnection_in_middle_of_a_frame() {
        let input = b"{\"ok\":";
        let error = read_frame(&mut BufReader::new(input.as_slice()))
            .await
            .unwrap_err();
        assert!(error.contains("closed"));
    }
}
