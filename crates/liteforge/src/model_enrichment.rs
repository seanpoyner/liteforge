//! Model enrichment: provider detection, capability inference, and source mapping.
//!
//! Ported from the legacy Python SDK's `pipelines/model_transform.py`.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::types::ModelCapabilities;

/// Provider detection result with display metadata.
#[derive(Debug, Clone)]
pub struct ModelEnrichment {
    pub provider: &'static str,
    pub provider_name: &'static str,
    pub source: &'static str,
    pub capabilities: ModelCapabilities,
}

// ---------------------------------------------------------------------------
// Provider pattern table  (order matters -- first match wins)
// ---------------------------------------------------------------------------

static PROVIDER_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        // OpenAI
        (Regex::new(r"(?i)^gpt-").unwrap(), "openai"),
        (Regex::new(r"(?i)^o1").unwrap(), "openai"),
        (Regex::new(r"(?i)^o3").unwrap(), "openai"),
        (Regex::new(r"(?i)^chatgpt-").unwrap(), "openai"),
        (Regex::new(r"(?i)^text-embedding-").unwrap(), "openai"),
        (Regex::new(r"(?i)^dall-e").unwrap(), "openai"),
        (Regex::new(r"(?i)^whisper").unwrap(), "openai"),
        (Regex::new(r"(?i)^openai\.").unwrap(), "openai"),
        // Anthropic
        (Regex::new(r"(?i)^claude-").unwrap(), "anthropic"),
        (Regex::new(r"(?i)^anthropic\.").unwrap(), "anthropic"),
        (Regex::new(r"(?i)^anthropic/").unwrap(), "anthropic"),
        // AWS Bedrock
        (Regex::new(r"(?i)^bedrock/").unwrap(), "aws-bedrock"),
        (Regex::new(r"(?i)^amazon\.").unwrap(), "aws-bedrock"),
        (Regex::new(r"(?i)^ai21\.").unwrap(), "aws-bedrock"),
        (Regex::new(r"(?i)^cohere\.").unwrap(), "aws-bedrock"),
        (Regex::new(r"(?i)^meta\.").unwrap(), "aws-bedrock"),
        (Regex::new(r"(?i)^mistral\.").unwrap(), "aws-bedrock"),
        (Regex::new(r"(?i)^stability\.").unwrap(), "aws-bedrock"),
        (Regex::new(r"(?i)^qwen\.").unwrap(), "aws-bedrock"),
        // Google
        (Regex::new(r"(?i)^gemini[/-]").unwrap(), "google"),
        (Regex::new(r"(?i)^palm-").unwrap(), "google"),
        (Regex::new(r"(?i)^google/").unwrap(), "google"),
        // Mistral (direct)
        (Regex::new(r"(?i)^mistral-").unwrap(), "mistral"),
        (Regex::new(r"(?i)^codestral").unwrap(), "mistral"),
        (Regex::new(r"(?i)^pixtral").unwrap(), "mistral"),
        // Meta (direct)
        (Regex::new(r"(?i)^llama-").unwrap(), "meta"),
        (Regex::new(r"(?i)^llama3").unwrap(), "meta"),
        // Cohere (direct)
        (Regex::new(r"(?i)^command-").unwrap(), "cohere"),
        (Regex::new(r"(?i)^embed-").unwrap(), "cohere"),
    ]
});

// ---------------------------------------------------------------------------
// Capability override patterns  (first match wins)
// ---------------------------------------------------------------------------

type CapApply = fn(&mut ModelCapabilities);

static CAP_OVERRIDES: Lazy<Vec<(Regex, CapApply)>> = Lazy::new(|| {
    vec![
        (Regex::new(r"(?i)^o1").unwrap(), (|c: &mut ModelCapabilities| {
            c.supports_thinking = true;
            c.thinking_style = Some("openai_reasoning".into());
            c.supports_vision = false;
        }) as CapApply),
        (Regex::new(r"(?i)^o3").unwrap(), |c| {
            c.supports_thinking = true;
            c.thinking_style = Some("openai_reasoning".into());
        }),
        (Regex::new(r"(?i)^gpt-3\.5").unwrap(), |c| {
            c.supports_vision = false;
        }),
        (Regex::new(r"(?i)^anthropic\.claude").unwrap(), |c| {
            c.supports_thinking = true;
            c.thinking_style = Some("anthropic".into());
            c.supports_vision = true;
        }),
        (Regex::new(r"(?i)^anthropic\.claude-3-haiku").unwrap(), |c| {
            c.supports_thinking = false;
            c.thinking_style = None;
        }),
        (Regex::new(r"(?i)^gemini-1\.5-flash-8b").unwrap(), |c| {
            c.context_window = Some(32_000);
        }),
        (Regex::new(r"(?i)^mistral\.mistral-7b").unwrap(), |c| {
            c.supports_tools = false;
        }),
        (Regex::new(r"(?i)^codestral").unwrap(), |c| {
            c.supports_vision = false;
        }),
        (Regex::new(r"(?i)^command-").unwrap(), |c| {
            c.supports_vision = false;
        }),
    ]
});

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Detect the provider identifier from a model ID (e.g. `"anthropic"`).
pub fn detect_provider(model_id: &str) -> &'static str {
    for (re, provider) in PROVIDER_PATTERNS.iter() {
        if re.is_match(model_id) {
            return provider;
        }
    }
    "unknown"
}

/// Map a provider identifier to its human-readable display name.
pub fn provider_name(provider: &str) -> &'static str {
    match provider {
        "openai" => "OpenAI",
        "anthropic" => "Anthropic",
        "aws-bedrock" => "AWS Bedrock",
        "google" => "Google",
        "mistral" => "Mistral",
        "cohere" => "Cohere",
        "meta" => "Meta",
        _ => "Other",
    }
}

/// Map a provider identifier to its cloud source (Azure, Bedrock, GCP, etc.).
pub fn provider_source(provider: &str) -> &'static str {
    match provider {
        "openai" => "Azure",
        "anthropic" => "Bedrock",
        "aws-bedrock" => "Bedrock",
        "google" => "GCP",
        "mistral" => "Bedrock",
        "meta" => "Bedrock",
        "cohere" => "Bedrock",
        _ => "Bedrock",
    }
}

/// Resolve capabilities for a model by layering provider defaults with
/// per-model overrides (matching the legacy Python SDK logic).
pub fn get_capabilities(model_id: &str) -> ModelCapabilities {
    let provider = detect_provider(model_id);

    let mut caps = provider_default_capabilities(provider);

    for (re, apply) in CAP_OVERRIDES.iter() {
        if re.is_match(model_id) {
            apply(&mut caps);
            break;
        }
    }

    caps
}

/// Fully enrich a model ID into provider, source, and capabilities.
pub fn enrich(model_id: &str) -> ModelEnrichment {
    let provider = detect_provider(model_id);
    ModelEnrichment {
        provider,
        provider_name: provider_name(provider),
        source: provider_source(provider),
        capabilities: get_capabilities(model_id),
    }
}

// ---------------------------------------------------------------------------
// Provider default capabilities
// ---------------------------------------------------------------------------

fn provider_default_capabilities(provider: &str) -> ModelCapabilities {
    match provider {
        "openai" => ModelCapabilities {
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
            supports_json_mode: true,
            context_window: Some(128_000),
            max_output_tokens: Some(16_384),
            ..Default::default()
        },
        "anthropic" => ModelCapabilities {
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
            supports_thinking: true,
            context_window: Some(200_000),
            max_output_tokens: Some(8_192),
            thinking_style: Some("anthropic".into()),
            ..Default::default()
        },
        "aws-bedrock" => ModelCapabilities {
            supports_tools: true,
            supports_streaming: true,
            ..Default::default()
        },
        "google" => ModelCapabilities {
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
            supports_json_mode: true,
            context_window: Some(1_000_000),
            max_output_tokens: Some(8_192),
            ..Default::default()
        },
        "mistral" => ModelCapabilities {
            supports_tools: true,
            supports_streaming: true,
            supports_json_mode: true,
            context_window: Some(128_000),
            max_output_tokens: Some(8_192),
            ..Default::default()
        },
        "cohere" => ModelCapabilities {
            supports_tools: true,
            supports_streaming: true,
            context_window: Some(128_000),
            max_output_tokens: Some(4_096),
            ..Default::default()
        },
        "meta" => ModelCapabilities {
            supports_tools: true,
            supports_streaming: true,
            context_window: Some(128_000),
            max_output_tokens: Some(4_096),
            ..Default::default()
        },
        _ => ModelCapabilities::default(),
    }
}

/// Format a context window size with thousands separators (e.g. `200,000`).
pub fn format_context(ctx: Option<u32>) -> String {
    match ctx {
        Some(n) => {
            let s = n.to_string();
            let mut result = String::with_capacity(s.len() + s.len() / 3);
            for (i, c) in s.chars().rev().enumerate() {
                if i > 0 && i % 3 == 0 {
                    result.push(',');
                }
                result.push(c);
            }
            result.chars().rev().collect()
        }
        None => "-".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_provider() {
        assert_eq!(detect_provider("gpt-4o"), "openai");
        assert_eq!(detect_provider("anthropic.claude-sonnet-4-20250514-v1:0"), "anthropic");
        assert_eq!(detect_provider("gemini-2.0-flash"), "google");
        assert_eq!(detect_provider("cohere.embed-english-v3"), "aws-bedrock");
        assert_eq!(detect_provider("command-r-plus"), "cohere");
        assert_eq!(detect_provider("meta.llama3-70b-instruct-v1:0"), "aws-bedrock");
        assert_eq!(detect_provider("llama-3.2"), "meta");
        assert_eq!(detect_provider("unknown-model"), "unknown");
    }

    #[test]
    fn test_provider_name() {
        assert_eq!(provider_name("openai"), "OpenAI");
        assert_eq!(provider_name("anthropic"), "Anthropic");
        assert_eq!(provider_name("unknown"), "Other");
    }

    #[test]
    fn test_provider_source() {
        assert_eq!(provider_source("openai"), "Azure");
        assert_eq!(provider_source("anthropic"), "Bedrock");
        assert_eq!(provider_source("google"), "GCP");
    }

    #[test]
    fn test_capabilities_anthropic_claude() {
        let caps = get_capabilities("anthropic.claude-sonnet-4-20250514-v1:0");
        assert!(caps.supports_thinking);
        assert!(caps.supports_vision);
        assert_eq!(caps.thinking_style.as_deref(), Some("anthropic"));
    }

    #[test]
    fn test_capabilities_openai_o1() {
        let caps = get_capabilities("o1-preview");
        assert!(caps.supports_thinking);
        assert!(!caps.supports_vision);
        assert_eq!(caps.thinking_style.as_deref(), Some("openai_reasoning"));
    }

    #[test]
    fn test_format_context() {
        assert_eq!(format_context(Some(200_000)), "200,000");
        assert_eq!(format_context(Some(128_000)), "128,000");
        assert_eq!(format_context(Some(1_000_000)), "1,000,000");
        assert_eq!(format_context(None), "-");
    }
}
