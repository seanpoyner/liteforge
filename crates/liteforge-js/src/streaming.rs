use crate::types::{rust_chunk_to_js, ChatCompletionChunk};
use futures::Stream;
use napi::bindgen_prelude::*;
use std::pin::Pin;
use std::sync::Arc;
use liteforge::ChatCompletionChunk as RustChatCompletionChunk;
use tokio::sync::Mutex;

type BoxedStream = Pin<Box<dyn Stream<Item = liteforge::Result<RustChatCompletionChunk>> + Send>>;

#[napi]
pub struct CompletionStream {
    stream: Arc<Mutex<Option<BoxedStream>>>,
}

impl CompletionStream {
    pub fn new(stream: BoxedStream) -> Self {
        Self {
            stream: Arc::new(Mutex::new(Some(stream))),
        }
    }
}

#[napi]
impl CompletionStream {
    #[napi]
    pub async fn next(&self) -> Result<Option<ChatCompletionChunk>> {
        use futures::StreamExt;
        let mut guard = self.stream.lock().await;
        let stream = match guard.as_mut() {
            Some(s) => s,
            None => return Ok(None),
        };
        match stream.next().await {
            Some(Ok(chunk)) => Ok(Some(rust_chunk_to_js(&chunk))),
            Some(Err(e)) => Err(crate::error::forge_error_to_napi(e)),
            None => {
                *guard = None;
                Ok(None)
            }
        }
    }
}
