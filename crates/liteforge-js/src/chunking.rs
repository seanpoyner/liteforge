use napi::bindgen_prelude::*;
use liteforge::{chunk as rust_chunk, Chunk as RustChunk, ChunkingStrategy as RustChunkingStrategy};

#[napi(object)]
pub struct JsChunk {
    pub text: String,
    pub index: u32,
    pub start_char: u32,
    pub end_char: u32,
}

fn rust_chunk_to_js(c: &RustChunk) -> JsChunk {
    JsChunk {
        text: c.text.clone(),
        index: c.index as u32,
        start_char: c.start_char as u32,
        end_char: c.end_char as u32,
    }
}

#[napi]
pub fn chunk(
    text: String,
    chunk_size: u32,
    overlap: u32,
    strategy: String,
) -> Result<Vec<JsChunk>> {
    let rust_strategy = match strategy.to_lowercase().as_str() {
        "fixed" => RustChunkingStrategy::Fixed,
        "recursive" => RustChunkingStrategy::Recursive,
        "sentence" => RustChunkingStrategy::Sentence,
        "paragraph" => RustChunkingStrategy::Paragraph,
        _ => {
            return Err(Error::from_reason(format!(
                "Unknown chunking strategy: '{}'. Use: fixed, recursive, sentence, paragraph",
                strategy
            )));
        }
    };
    let chunks = rust_chunk(&text, chunk_size as usize, overlap as usize, rust_strategy);
    Ok(chunks.iter().map(rust_chunk_to_js).collect())
}
