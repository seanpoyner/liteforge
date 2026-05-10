use std::path::Path;

use crate::error::CliError;

/// Extract text content from a file, dispatching to the appropriate
/// parser based on file extension.
pub fn extract_text(path: &str) -> Result<String, CliError> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "pdf" => extract_pdf(path),
        "docx" | "xlsx" | "pptx" => extract_office(path),
        "csv" => extract_csv(path),
        _ => std::fs::read_to_string(path).map_err(CliError::Io),
    }
}

fn extract_pdf(path: &str) -> Result<String, CliError> {
    let bytes = std::fs::read(path).map_err(CliError::Io)?;
    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| CliError::Extraction(format!("Failed to extract text from PDF: {}", e)))
}

fn extract_office(path: &str) -> Result<String, CliError> {
    undoc::extract_text(path)
        .map_err(|e| CliError::Extraction(format!("Failed to extract text from document: {}", e)))
}

fn extract_csv(path: &str) -> Result<String, CliError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .map_err(|e| CliError::Extraction(format!("Failed to read CSV: {}", e)))?;

    let mut lines = Vec::new();

    if let Ok(headers) = reader.headers().cloned() {
        lines.push(headers.iter().collect::<Vec<_>>().join(", "));
    }

    for result in reader.records() {
        let record = result
            .map_err(|e| CliError::Extraction(format!("Failed to read CSV row: {}", e)))?;
        lines.push(record.iter().collect::<Vec<_>>().join(", "));
    }

    Ok(lines.join("\n"))
}
