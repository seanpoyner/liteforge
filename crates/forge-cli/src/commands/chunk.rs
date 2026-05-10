use crate::error::CliError;
use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

#[derive(clap::Args)]
pub struct ChunkArgs {
    /// File to chunk
    pub file: String,

    /// Chunk size in characters
    #[arg(short, long, default_value = "500")]
    pub size: usize,

    /// Overlap between chunks in characters
    #[arg(short, long, default_value = "50")]
    pub overlap: usize,

    /// Chunking strategy
    #[arg(long, value_enum, default_value = "recursive")]
    pub strategy: ChunkStrategy,

    /// Output as JSON array
    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum ChunkStrategy {
    /// Fixed-size chunks
    Fixed,
    /// Recursive splitting on separators
    Recursive,
    /// Split on sentence boundaries
    Sentence,
    /// Split on paragraph boundaries
    Paragraph,
}

#[derive(Serialize)]
struct ChunkOutput {
    index: usize,
    start: usize,
    end: usize,
    length: usize,
    text: String,
}

pub async fn execute(args: ChunkArgs) -> Result<(), CliError> {
    let content = std::fs::read_to_string(&args.file).map_err(CliError::Io)?;

    let chunks = match args.strategy {
        ChunkStrategy::Fixed => chunk_fixed(&content, args.size, args.overlap),
        ChunkStrategy::Recursive => chunk_recursive(&content, args.size, args.overlap),
        ChunkStrategy::Sentence => chunk_sentence(&content, args.size, args.overlap),
        ChunkStrategy::Paragraph => chunk_paragraph(&content, args.size, args.overlap),
    };

    if args.json {
        let output: Vec<ChunkOutput> = chunks
            .iter()
            .enumerate()
            .map(|(i, (start, end, text))| ChunkOutput {
                index: i,
                start: *start,
                end: *end,
                length: text.len(),
                text: text.clone(),
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("File: {}", args.file);
        println!("Strategy: {:?}", args.strategy as u8);
        println!("Chunk size: {}, Overlap: {}", args.size, args.overlap);
        println!("Total chunks: {}\n", chunks.len());

        for (i, (start, end, text)) in chunks.iter().enumerate() {
            println!(
                "--- Chunk {} [{}-{}] ({} chars) ---",
                i + 1,
                start,
                end,
                text.len()
            );
            // Show preview (first 100 chars)
            let preview: String = text.chars().take(100).collect();
            if text.len() > 100 {
                println!("{}...", preview);
            } else {
                println!("{}", preview);
            }
            println!();
        }
    }

    Ok(())
}

fn chunk_fixed(text: &str, size: usize, overlap: usize) -> Vec<(usize, usize, String)> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let step = size.saturating_sub(overlap).max(1);
    let mut pos = 0;

    while pos < chars.len() {
        let end = (pos + size).min(chars.len());
        let chunk: String = chars[pos..end].iter().collect();
        chunks.push((pos, end, chunk));
        pos += step;
    }

    chunks
}

fn chunk_recursive(text: &str, size: usize, overlap: usize) -> Vec<(usize, usize, String)> {
    let separators = ["\n\n", "\n", ". ", " "];
    recursive_split(text, 0, &separators, size, overlap)
}

fn recursive_split(
    text: &str,
    offset: usize,
    separators: &[&str],
    size: usize,
    overlap: usize,
) -> Vec<(usize, usize, String)> {
    if text.len() <= size {
        return vec![(offset, offset + text.len(), text.to_string())];
    }

    if separators.is_empty() {
        return chunk_fixed(text, size, overlap)
            .into_iter()
            .map(|(s, e, t)| (offset + s, offset + e, t))
            .collect();
    }

    let sep = separators[0];
    let parts: Vec<&str> = text.split(sep).collect();

    if parts.len() == 1 {
        return recursive_split(text, offset, &separators[1..], size, overlap);
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_start = offset;

    for (i, part) in parts.iter().enumerate() {
        let addition = if i == 0 {
            part.to_string()
        } else {
            format!("{}{}", sep, part)
        };

        if current.len() + addition.len() > size && !current.is_empty() {
            chunks.push((
                current_start,
                current_start + current.len(),
                current.clone(),
            ));

            // Start new chunk with overlap
            let overlap_chars = current.chars().rev().take(overlap).collect::<Vec<_>>();
            current = overlap_chars.into_iter().rev().collect();
            current_start = current_start + current.len() - overlap;
            current.push_str(&addition);
        } else {
            current.push_str(&addition);
        }
    }

    if !current.is_empty() {
        chunks.push((current_start, current_start + current.len(), current));
    }

    chunks
}

fn chunk_sentence(text: &str, size: usize, overlap: usize) -> Vec<(usize, usize, String)> {
    let sentences: Vec<&str> = text.unicode_sentences().collect();
    merge_with_size(&sentences, size, overlap)
}

fn chunk_paragraph(text: &str, size: usize, overlap: usize) -> Vec<(usize, usize, String)> {
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .collect();
    merge_with_size(&paragraphs, size, overlap)
}

fn merge_with_size(parts: &[&str], size: usize, overlap: usize) -> Vec<(usize, usize, String)> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_start: usize = 0;
    let mut pos: usize = 0;

    for part in parts {
        if current.len() + part.len() > size && !current.is_empty() {
            chunks.push((
                current_start,
                current_start + current.len(),
                current.clone(),
            ));

            let overlap_chars = current.chars().rev().take(overlap).collect::<Vec<_>>();
            current = overlap_chars.into_iter().rev().collect();
            current_start = pos.saturating_sub(overlap);
        }

        if !current.is_empty() && !current.ends_with(' ') && !current.ends_with('\n') {
            current.push(' ');
        }
        current.push_str(part);
        pos += part.len() + 1;
    }

    if !current.is_empty() {
        chunks.push((current_start, current_start + current.len(), current));
    }

    chunks
}
