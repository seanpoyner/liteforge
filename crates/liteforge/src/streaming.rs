//! SSE streaming support for chat completions.

use crate::error::{Result, ForgeError};
use crate::types::ChatCompletionChunk;
use futures::Stream;
use tokio_stream::StreamExt;

/// Parse a single SSE line into a chunk.
///
/// Returns `Ok(Some(chunk))` for data lines, `Ok(None)` for [DONE] or non-data lines,
/// and `Err` for parse errors.
pub fn parse_sse_line(line: &str) -> Result<Option<ChatCompletionChunk>> {
    let line = line.trim();

    // Skip empty lines and comments
    if line.is_empty() || line.starts_with(':') {
        return Ok(None);
    }

    // Check for data: prefix
    if let Some(data) = line.strip_prefix("data:") {
        let data = data.trim();

        // Handle [DONE] sentinel
        if data == "[DONE]" {
            return Ok(None);
        }

        // Parse JSON
        let chunk: ChatCompletionChunk = serde_json::from_str(data)?;
        return Ok(Some(chunk));
    }

    // Skip other SSE fields (event:, id:, retry:)
    Ok(None)
}

/// Create a stream of chunks from an SSE byte stream.
pub fn parse_sse_stream(
    byte_stream: impl Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Unpin,
) -> impl Stream<Item = Result<ChatCompletionChunk>> {
    async_stream::stream! {
        let mut buffer = String::new();
        let mut stream = byte_stream;

        while let Some(result) = stream.next().await {
            match result {
                Ok(bytes) => {
                    // Append bytes to buffer
                    match std::str::from_utf8(&bytes) {
                        Ok(text) => buffer.push_str(text),
                        Err(e) => {
                            yield Err(ForgeError::stream(format!("Invalid UTF-8: {}", e)));
                            continue;
                        }
                    }

                    // Process complete lines
                    while let Some(pos) = buffer.find('\n') {
                        let line: String = buffer.drain(..=pos).collect();

                        match parse_sse_line(&line) {
                            Ok(Some(chunk)) => yield Ok(chunk),
                            Ok(None) => {} // Skip non-data lines
                            Err(e) => yield Err(e),
                        }
                    }
                }
                Err(e) => {
                    yield Err(ForgeError::from(e));
                }
            }
        }

        // Process any remaining data in buffer
        if !buffer.is_empty() {
            match parse_sse_line(&buffer) {
                Ok(Some(chunk)) => yield Ok(chunk),
                Ok(None) => {}
                Err(e) => yield Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_done() {
        assert!(parse_sse_line("data: [DONE]").unwrap().is_none());
    }

    #[test]
    fn test_parse_sse_empty() {
        assert!(parse_sse_line("").unwrap().is_none());
        assert!(parse_sse_line(": comment").unwrap().is_none());
    }

    #[test]
    fn test_parse_sse_data() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let chunk = parse_sse_line(line).unwrap().unwrap();
        assert_eq!(chunk.id, "chatcmpl-123");
        assert_eq!(chunk.content(), Some("Hello"));
    }
}
