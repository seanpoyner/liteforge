use crate::error::CliError;
use crate::input::resolve_prompt;
use crate::output::OutputFormat;
use serde::Serialize;
use liteforge::{AsyncForgeClient, ForgeConfig};

#[derive(clap::Args)]
pub struct EmbedArgs {
    /// Text to embed (reads from stdin if omitted)
    pub text: Option<String>,

    /// Model to use for embedding
    #[arg(short, long)]
    pub model: Option<String>,

    /// Read text from a file
    #[arg(short, long)]
    pub file: Option<String>,

    /// Read text from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub output: OutputFormat,

    /// API key override
    #[arg(long, env = "LITEFORGE_API_KEY")]
    pub api_key: Option<String>,

    /// Base URL override
    #[arg(long, env = "LITEFORGE_BASE_URL")]
    pub base_url: Option<String>,
}

#[derive(Serialize)]
struct EmbedOutput {
    model: String,
    dimensions: usize,
    embedding: Vec<f32>,
    usage: EmbedUsage,
}

#[derive(Serialize)]
struct EmbedUsage {
    prompt_tokens: u32,
    total_tokens: u32,
}

pub async fn execute(args: EmbedArgs) -> Result<(), CliError> {
    let mut config_builder = ForgeConfig::builder();
    if let Some(key) = &args.api_key {
        config_builder = config_builder.api_key(key.clone());
    }
    if let Some(url) = &args.base_url {
        config_builder = config_builder.base_url(url.clone());
    }
    if let Some(model) = &args.model {
        config_builder = config_builder.default_model(model.clone());
    }

    let config = config_builder.build();
    let client = AsyncForgeClient::with_config(config);

    let text = resolve_prompt(args.text, args.file, args.stdin)?;
    let response = client.embed(text).await?;

    match args.output {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        }
        OutputFormat::Pretty | OutputFormat::Raw => {
            if let Some(data) = response.data.first() {
                let output = EmbedOutput {
                    model: response.model.clone(),
                    dimensions: data.embedding.len(),
                    embedding: data.embedding.clone(),
                    usage: EmbedUsage {
                        prompt_tokens: response.usage.prompt_tokens,
                        total_tokens: response.usage.total_tokens,
                    },
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
        }
    }

    Ok(())
}
