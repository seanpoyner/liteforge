//! Text chunking utilities for RAG pipelines.
//!
//! This module provides functions to split documents into retrievable segments
//! with various chunking strategies.
//!
//! # Example
//!
//! ```
//! use liteforge::chunking::{chunk, ChunkingStrategy};
//!
//! let text = "Long document with multiple paragraphs...";
//! let chunks = chunk(text, 500, 50, ChunkingStrategy::Recursive);
//! println!("Created {} chunks", chunks.len());
//! ```

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

/// Available chunking strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChunkingStrategy {
    /// Fixed size chunks with overlap
    Fixed,
    /// Split by separators recursively (paragraphs -> lines -> sentences -> words)
    Recursive,
    /// Split by sentence boundaries
    Sentence,
    /// Split by paragraph boundaries
    Paragraph,
}

impl std::str::FromStr for ChunkingStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fixed" => Ok(ChunkingStrategy::Fixed),
            "recursive" => Ok(ChunkingStrategy::Recursive),
            "sentence" => Ok(ChunkingStrategy::Sentence),
            "paragraph" => Ok(ChunkingStrategy::Paragraph),
            _ => Err(format!("Unknown strategy: {}", s)),
        }
    }
}

/// A chunk of text with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// The chunk content
    pub text: String,
    /// Position in the original document (0-indexed)
    pub index: usize,
    /// Starting character position in original text
    pub start_char: usize,
    /// Ending character position in original text
    pub end_char: usize,
}

impl Chunk {
    /// Create a new chunk
    pub fn new(text: String, index: usize, start_char: usize, end_char: usize) -> Self {
        Self {
            text,
            index,
            start_char,
            end_char,
        }
    }

    /// Get the length of the chunk text
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Check if the chunk is empty
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// Default separators for recursive chunking (ordered by priority)
const DEFAULT_SEPARATORS: &[&str] = &[
    "\n\n", // Paragraphs
    "\n",   // Lines
    ". ",   // Sentences
    ", ",   // Clauses
    " ",    // Words
];

/// Split text into chunks for RAG retrieval.
///
/// # Arguments
///
/// * `text` - The text to chunk
/// * `chunk_size` - Target size for each chunk in characters
/// * `overlap` - Number of overlapping characters between chunks
/// * `strategy` - Chunking strategy to use
///
/// # Returns
///
/// A vector of Chunk objects with text and position metadata.
///
/// # Example
///
/// ```
/// use liteforge::chunking::{chunk, ChunkingStrategy};
///
/// let chunks = chunk("Hello world. This is a test.", 20, 5, ChunkingStrategy::Sentence);
/// for c in chunks {
///     println!("Chunk {}: {}", c.index, c.text);
/// }
/// ```
pub fn chunk(
    text: &str,
    chunk_size: usize,
    overlap: usize,
    strategy: ChunkingStrategy,
) -> Vec<Chunk> {
    if text.is_empty() || text.trim().is_empty() {
        return vec![];
    }

    match strategy {
        ChunkingStrategy::Fixed => chunk_fixed(text, chunk_size, overlap),
        ChunkingStrategy::Recursive => {
            chunk_recursive(text, chunk_size, overlap, DEFAULT_SEPARATORS)
        }
        ChunkingStrategy::Sentence => chunk_by_sentence(text, chunk_size, overlap),
        ChunkingStrategy::Paragraph => chunk_by_paragraph(text, chunk_size, overlap),
    }
}

/// Split text into fixed-size chunks.
fn chunk_fixed(text: &str, chunk_size: usize, overlap: usize) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut start: usize = 0;
    let mut index: usize = 0;

    while start < text.len() {
        let end = (start + chunk_size).min(text.len());
        let chunk_text = &text[start..end];

        if !chunk_text.trim().is_empty() {
            chunks.push(Chunk::new(chunk_text.to_string(), index, start, end));
            index += 1;
        }

        if end >= text.len() {
            break;
        }
        start = if overlap < chunk_size {
            end - overlap
        } else {
            end
        };
    }

    chunks
}

/// Split text recursively using separators.
fn chunk_recursive(
    text: &str,
    chunk_size: usize,
    overlap: usize,
    separators: &[&str],
) -> Vec<Chunk> {
    if text.len() <= chunk_size {
        return vec![Chunk::new(text.to_string(), 0, 0, text.len())];
    }

    // Find the first separator that exists in the text
    let sep_to_use = separators.iter().find(|&&sep| text.contains(sep));

    match sep_to_use {
        None | Some(&"") => chunk_fixed(text, chunk_size, overlap),
        Some(&sep) => {
            let parts: Vec<&str> = text.split(sep).collect();
            merge_parts_into_chunks(&parts, sep, chunk_size, overlap)
        }
    }
}

/// Merge split parts back into appropriately sized chunks.
fn merge_parts_into_chunks(
    parts: &[&str],
    sep: &str,
    chunk_size: usize,
    overlap: usize,
) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_start: usize = 0;
    let mut index: usize = 0;
    let mut char_pos: usize = 0;

    for (i, part) in parts.iter().enumerate() {
        let part_with_sep = if i < parts.len() - 1 {
            format!("{}{}", part, sep)
        } else {
            part.to_string()
        };

        if current_chunk.len() + part_with_sep.len() <= chunk_size {
            current_chunk.push_str(&part_with_sep);
        } else {
            // Save current chunk if not empty
            let trimmed = current_chunk.trim();
            if !trimmed.is_empty() {
                chunks.push(Chunk::new(
                    trimmed.to_string(),
                    index,
                    current_start,
                    current_start + current_chunk.len(),
                ));
                index += 1;
            }

            // Handle overlap
            if overlap > 0 && !current_chunk.is_empty() {
                let overlap_start = current_chunk.len().saturating_sub(overlap);
                let overlap_text = current_chunk[overlap_start..].to_string();
                let overlap_len = overlap_text.len();
                current_chunk = format!("{}{}", overlap_text, part_with_sep);
                current_start = char_pos.saturating_sub(overlap_len);
            } else {
                current_chunk = part_with_sep.clone();
                current_start = char_pos;
            }
        }

        char_pos += part_with_sep.len();
    }

    // Add final chunk
    let trimmed = current_chunk.trim();
    if !trimmed.is_empty() {
        chunks.push(Chunk::new(
            trimmed.to_string(),
            index,
            current_start,
            current_start + current_chunk.len(),
        ));
    }

    chunks
}

/// Split text by sentences, then combine into chunks.
fn chunk_by_sentence(text: &str, chunk_size: usize, _overlap: usize) -> Vec<Chunk> {
    let sentences: Vec<&str> = text.unicode_sentences().collect();

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_start: usize = 0;
    let mut index: usize = 0;
    let mut char_pos: usize = 0;

    for sentence in sentences {
        let sentence = sentence.trim();
        if sentence.is_empty() {
            continue;
        }

        let with_space = if current_chunk.is_empty() {
            sentence.to_string()
        } else {
            format!(" {}", sentence)
        };

        if current_chunk.len() + with_space.len() <= chunk_size {
            current_chunk.push_str(&with_space);
        } else {
            if !current_chunk.is_empty() {
                chunks.push(Chunk::new(
                    current_chunk.clone(),
                    index,
                    current_start,
                    char_pos,
                ));
                index += 1;
            }

            current_chunk = sentence.to_string();
            current_start = char_pos;
        }

        char_pos += sentence.len() + 1;
    }

    if !current_chunk.is_empty() {
        chunks.push(Chunk::new(current_chunk, index, current_start, char_pos));
    }

    chunks
}

/// Split text by paragraphs, then combine into chunks.
fn chunk_by_paragraph(text: &str, chunk_size: usize, overlap: usize) -> Vec<Chunk> {
    let paragraphs: Vec<&str> = text.split("\n\n").collect();

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_start: usize = 0;
    let mut index: usize = 0;
    let mut char_pos: usize = 0;

    for para in paragraphs {
        let para = para.trim();
        if para.is_empty() {
            char_pos += 2; // Account for \n\n
            continue;
        }

        let with_sep = if current_chunk.is_empty() {
            para.to_string()
        } else {
            format!("\n\n{}", para)
        };

        if current_chunk.len() + with_sep.len() <= chunk_size {
            current_chunk.push_str(&with_sep);
        } else {
            if !current_chunk.is_empty() {
                chunks.push(Chunk::new(
                    current_chunk.clone(),
                    index,
                    current_start,
                    char_pos,
                ));
                index += 1;
            }

            // If paragraph itself is too large, chunk it recursively
            if para.len() > chunk_size {
                let sub_chunks = chunk_recursive(para, chunk_size, overlap, DEFAULT_SEPARATORS);
                for mut sc in sub_chunks {
                    sc.index = index;
                    sc.start_char += char_pos;
                    sc.end_char += char_pos;
                    chunks.push(sc);
                    index += 1;
                }
                current_chunk = String::new();
                current_start = char_pos + para.len() + 2;
            } else {
                current_chunk = para.to_string();
                current_start = char_pos;
            }
        }

        char_pos += para.len() + 2;
    }

    if !current_chunk.is_empty() {
        chunks.push(Chunk::new(current_chunk, index, current_start, char_pos));
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_chunking() {
        let text = "Hello world, this is a test of chunking.";
        let chunks = chunk(text, 15, 5, ChunkingStrategy::Fixed);
        assert!(!chunks.is_empty());
        assert!(chunks[0].len() <= 15);
    }

    #[test]
    fn test_sentence_chunking() {
        let text = "First sentence. Second sentence. Third sentence.";
        let chunks = chunk(text, 30, 0, ChunkingStrategy::Sentence);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_empty_text() {
        let chunks = chunk("", 100, 10, ChunkingStrategy::Fixed);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_strategy_from_str() {
        assert_eq!(
            "fixed".parse::<ChunkingStrategy>().unwrap(),
            ChunkingStrategy::Fixed
        );
        assert_eq!(
            "sentence".parse::<ChunkingStrategy>().unwrap(),
            ChunkingStrategy::Sentence
        );
    }
}
