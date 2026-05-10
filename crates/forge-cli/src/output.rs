use crate::theme;
use liteforge::{ChatCompletion, Model, ModelList};

#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Pretty,
    Json,
    Raw,
}

fn detect_provider(id: &str) -> &str {
    if id.starts_with("anthropic.") || id.contains("claude") {
        "Anthropic"
    } else if id.starts_with("amazon.") || id.contains("titan") || id.contains("nova") {
        "Amazon"
    } else if id.starts_with("cohere.") {
        "Cohere"
    } else if id.starts_with("gemini/") || id.starts_with("gemini-") {
        "Google"
    } else if id.starts_with("gpt-") || id.starts_with("o1") || id.starts_with("o3") {
        "OpenAI"
    } else if id.starts_with("mistral") {
        "Mistral"
    } else {
        "Other"
    }
}

fn detect_source(id: &str) -> &str {
    if id.starts_with("gemini/") {
        "Vertex AI"
    } else if id.contains('.')
        && (id.contains(":0")
            || id.starts_with("amazon.")
            || id.starts_with("anthropic.")
            || id.starts_with("cohere."))
    {
        "Bedrock"
    } else {
        "API"
    }
}

fn has_capability(id: &str, cap: &str) -> bool {
    match cap {
        "tools" => {
            id.contains("claude")
                || id.starts_with("gpt-")
                || id.contains("nova-pro")
                || id.contains("nova-premier")
                || id.contains("gemini")
                || id.starts_with("o1")
                || id.starts_with("o3")
        }
        "vision" => {
            id.contains("claude") && !id.contains("haiku-20240307")
                || id.starts_with("gpt-4")
                || id.contains("nova-pro")
                || id.contains("nova-premier")
                || (id.contains("gemini") && !id.contains("embed"))
        }
        "thinking" => {
            id.contains("claude-3-7")
                || id.contains("claude-sonnet-4")
                || id.contains("claude-opus-4")
                || id.contains("claude-haiku-4")
                || id.starts_with("o1")
                || id.starts_with("o3")
                || id.contains("gemini-2.5")
        }
        _ => false,
    }
}

fn is_embedding(id: &str) -> bool {
    id.contains("embed") || id.contains("ada-002")
}

pub fn print_completion(completion: &ChatCompletion, format: OutputFormat) {
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

            let mut chat_models: Vec<&Model> = Vec::new();
            let mut embed_models: Vec<&Model> = Vec::new();
            for model in &models.data {
                if is_embedding(&model.id) {
                    embed_models.push(model);
                } else {
                    chat_models.push(model);
                }
            }

            if !chat_models.is_empty() {
                println!("  {}", theme::subheader("Chat & Reasoning"));
                println!();
                for model in &chat_models {
                    let provider = detect_provider(&model.id);
                    let source = detect_source(&model.id);
                    let tools = has_capability(&model.id, "tools");
                    let vision = has_capability(&model.id, "vision");
                    let thinking = has_capability(&model.id, "thinking");

                    println!(
                        "  {} {} {} {} {} {}",
                        color_pad(&model.id, 48, theme::value),
                        color_pad(provider, 10, theme::dimmed),
                        color_pad(source, 12, theme::dimmed),
                        if tools {
                            color_pad("Y", 6, theme::success)
                        } else {
                            color_pad("-", 6, theme::dimmed)
                        },
                        if vision {
                            color_pad("Y", 6, theme::success)
                        } else {
                            color_pad("-", 6, theme::dimmed)
                        },
                        if thinking {
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
                for model in &embed_models {
                    let provider = detect_provider(&model.id);
                    let source = detect_source(&model.id);
                    println!(
                        "  {} {} {} {} {} {}",
                        color_pad(&model.id, 48, theme::pink),
                        color_pad(provider, 10, theme::dimmed),
                        color_pad(source, 12, theme::dimmed),
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
                theme::success(&format!("{}", models.data.len())),
                theme::dimmed("models available"),
            );
            println!();
            println!("  {}", theme::gradient_divider(94));
            println!();
        }
    }
}

pub fn print_model_info(model: &Model, is_default: bool) {
    let provider = detect_provider(&model.id);
    let source = detect_source(&model.id);

    println!();
    println!("  {}", theme::gradient_divider(50));
    println!();

    crate::ui::kv_value("ID", &model.id);
    crate::ui::kv("Provider", &theme::value(provider));
    crate::ui::kv("Source", &theme::value(source));
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
        crate::ui::kv("Tools", &theme::yn(has_capability(&model.id, "tools")));
        crate::ui::kv("Vision", &theme::yn(has_capability(&model.id, "vision")));
        crate::ui::kv(
            "Thinking",
            &theme::yn(has_capability(&model.id, "thinking")),
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
