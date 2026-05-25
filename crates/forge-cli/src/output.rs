use crate::theme;
use liteforge::{Model, ModelList};
use liteforge::model_enrichment::{enrich, ModelEnrichment};
use std::collections::HashMap;

/// Canonical key used to detect alias pairs like
/// `claude-opus-4.7` / `claude-opus-4-7`, or `gemini-3.1-pro` / `models/gemini-3.1-pro`.
fn canonical_id(id: &str) -> String {
    let stripped = id.strip_prefix("models/").unwrap_or(id);
    stripped.replace('.', "-")
}

/// Lower score = preferred display ID. We prefer no `models/` prefix and
/// dotted version numbers (more readable than dash-only forms).
fn display_preference(id: &str) -> i32 {
    let mut score = 0;
    if id.starts_with("models/") {
        score += 10;
    }
    if !id.contains('.') {
        score += 1;
    }
    score
}

#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Pretty,
    Json,
    Raw,
}

pub fn print_completion(completion: &liteforge::ChatCompletion, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(completion).unwrap());
        }
        OutputFormat::Raw => {
            if let Some(content) = completion.content() {
                print!("{}", content);
            }
        }
        OutputFormat::Pretty => {
            if let Some(content) = completion.content() {
                println!("{}", content);
            }

            if let Some(usage) = &completion.usage {
                eprintln!(
                    "\n  {}",
                    theme::dimmed(&format!(
                        "[{} | prompt: {} | completion: {} | total: {} tokens]",
                        completion.model,
                        usage.prompt_tokens,
                        usage.completion_tokens,
                        usage.total_tokens,
                    ))
                );
            }
        }
    }
}

fn pad(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - s.len()))
    }
}

fn color_pad(text: &str, width: usize, colorize: fn(&str) -> String) -> String {
    colorize(&pad(text, width))
}

pub fn print_models(models: &ModelList, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(models).unwrap());
        }
        OutputFormat::Raw => {
            for model in &models.data {
                println!("{}", model.id);
            }
        }
        OutputFormat::Pretty => {
            println!();
            println!("  {}", theme::gradient_divider(94));
            println!();

            println!(
                "  {} {} {} {} {} {}",
                color_pad("MODEL ID", 48, theme::label),
                color_pad("PROVIDER", 10, theme::label),
                color_pad("SOURCE", 12, theme::label),
                color_pad("TOOLS", 6, theme::label),
                color_pad("VISION", 6, theme::label),
                color_pad("THINK", 6, theme::label),
            );
            println!("  {}", theme::dimmed(&"─".repeat(94)));

            // Deduplicate aliases (e.g. `claude-opus-4.7` vs `claude-opus-4-7`,
            // `gemini-3.1-pro` vs `models/gemini-3.1-pro`) by canonical key,
            // preserving the order of first appearance.
            let mut by_canonical: HashMap<String, usize> = HashMap::new();
            let mut unique_models: Vec<&Model> = Vec::new();

            for model in &models.data {
                let key = canonical_id(&model.id);
                if let Some(&idx) = by_canonical.get(&key) {
                    let existing: &Model = unique_models[idx];
                    if display_preference(&model.id) < display_preference(&existing.id) {
                        unique_models[idx] = model;
                    }
                } else {
                    by_canonical.insert(key, unique_models.len());
                    unique_models.push(model);
                }
            }
            
            // Separate chat and embedding models
            let mut chat_models: Vec<(&Model, ModelEnrichment)> = Vec::new();
            let mut embed_models: Vec<(&Model, ModelEnrichment)> = Vec::new();
            
            for model in &unique_models {
                let enrichment = enrich(&model.id);
                if is_embedding(&model.id) {
                    embed_models.push((model, enrichment));
                } else {
                    chat_models.push((model, enrichment));
                }
            }

            if !chat_models.is_empty() {
                println!("  {}", theme::subheader("Chat & Reasoning"));
                println!();
                for (model, enrichment) in &chat_models {
                    println!(
                        "  {} {} {} {} {} {}",
                        color_pad(&model.id, 48, theme::value),
                        color_pad(enrichment.provider_name, 10, theme::dimmed),
                        color_pad(enrichment.source, 12, theme::dimmed),
                        if enrichment.capabilities.supports_tools {
                            color_pad("Y", 6, theme::success)
                        } else {
                            color_pad("-", 6, theme::dimmed)
                        },
                        if enrichment.capabilities.supports_vision {
                            color_pad("Y", 6, theme::success)
                        } else {
                            color_pad("-", 6, theme::dimmed)
                        },
                        if enrichment.capabilities.supports_thinking {
                            color_pad("Y", 6, theme::success)
                        } else {
                            color_pad("-", 6, theme::dimmed)
                        },
                    );
                }
            }

            if !embed_models.is_empty() {
                println!();
                println!("  {}", theme::subheader("Embeddings"));
                println!();
                for (model, enrichment) in &embed_models {
                    println!(
                        "  {} {} {} {} {} {}",
                        color_pad(&model.id, 48, theme::pink),
                        color_pad(enrichment.provider_name, 10, theme::dimmed),
                        color_pad(enrichment.source, 12, theme::dimmed),
                        color_pad("-", 6, theme::dimmed),
                        color_pad("-", 6, theme::dimmed),
                        color_pad("-", 6, theme::dimmed),
                    );
                }
            }

            println!();
            println!("  {}", theme::dimmed(&"─".repeat(94)));
            println!(
                "  {} {}",
                theme::success(&format!("{}", unique_models.len())),
                theme::dimmed("models available"),
            );
            println!();
            println!("  {}", theme::gradient_divider(94));
            println!();
        }
    }
}

fn is_embedding(id: &str) -> bool {
    id.contains("embed") || id.contains("ada-002")
}

pub fn print_model_info(model: &Model, is_default: bool) {
    let enrichment = enrich(&model.id);

    println!();
    println!("  {}", theme::gradient_divider(50));
    println!();

    crate::ui::kv_value("ID", &model.id);
    crate::ui::kv("Provider", &theme::value(enrichment.provider_name));
    crate::ui::kv("Source", &theme::value(enrichment.source));
    crate::ui::kv(
        "Owner",
        &theme::dimmed(model.owned_by.as_deref().unwrap_or("-")),
    );
    crate::ui::kv(
        "Created",
        &theme::dimmed(
            &model
                .created
                .map(|c| c.to_string())
                .unwrap_or_else(|| "-".to_string()),
        ),
    );

    if !is_embedding(&model.id) {
        println!();
        crate::ui::kv("Tools", &theme::yn(enrichment.capabilities.supports_tools));
        crate::ui::kv("Vision", &theme::yn(enrichment.capabilities.supports_vision));
        crate::ui::kv(
            "Thinking",
            &theme::yn(enrichment.capabilities.supports_thinking),
        );
        crate::ui::kv(
            "Context Window",
            &theme::value(&liteforge::model_enrichment::format_context(enrichment.capabilities.context_window)),
        );
        crate::ui::kv(
            "Max Output Tokens",
            &theme::value(&liteforge::model_enrichment::format_context(enrichment.capabilities.max_output_tokens)),
        );
    }

    if is_default {
        println!();
        crate::ui::kv_success("Default", "yes");
    }

    println!();
    println!("  {}", theme::gradient_divider(50));
    println!();
}

pub fn print_config(api_key: Option<&str>, base_url: &str, model: &str, timeout_secs: u64) {
    let masked_key = match api_key {
        Some(k) if k.len() > 8 => {
            format!("{}...{}", &k[..4], &k[k.len() - 4..])
        }
        Some(_) => "****".to_string(),
        None => theme::error_text("(not set)"),
    };

    println!();
    println!("  {}", theme::gradient_divider(50));
    println!();
    crate::ui::kv("API Key", &masked_key);
    crate::ui::kv_value("Base URL", base_url);
    crate::ui::kv_value("Model", model);
    crate::ui::kv(
        "Timeout",
        &format!("{}s", theme::value(&timeout_secs.to_string())),
    );
    println!();
    println!("  {}", theme::gradient_divider(50));
    println!();
}
