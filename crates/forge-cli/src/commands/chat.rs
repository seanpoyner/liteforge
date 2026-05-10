use crate::error::CliError;
use crate::input::resolve_prompt;
use crate::output::{self, OutputFormat};
use crate::{logo, theme, ui};
use futures::StreamExt;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::Write;
use liteforge::{AsyncForgeClient, ChatCompletionRequest, Message, ForgeConfig};

#[derive(clap::Args)]
pub struct ChatArgs {
    /// The prompt to send (reads from stdin if omitted)
    pub prompt: Option<String>,

    /// Model to use for completion
    #[arg(short, long)]
    pub model: Option<String>,

    /// System prompt to prepend
    #[arg(short, long)]
    pub system: Option<String>,

    /// Sampling temperature (0.0 to 2.0)
    #[arg(short = 'T', long)]
    pub temperature: Option<f32>,

    /// Maximum tokens to generate
    #[arg(long)]
    pub max_tokens: Option<u32>,

    /// Enable streaming output
    #[arg(long)]
    pub stream: bool,

    /// Interactive chat mode
    #[arg(short, long)]
    pub interactive: bool,

    /// Read prompt from a file
    #[arg(short, long)]
    pub file: Option<String>,

    /// Read prompt from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Output format
    #[arg(short, long, value_enum, default_value = "pretty")]
    pub output: OutputFormat,

    /// API key override
    #[arg(long, env = "LITEFORGE_API_KEY")]
    pub api_key: Option<String>,

    /// Base URL override
    #[arg(long, env = "LITEFORGE_BASE_URL")]
    pub base_url: Option<String>,
}

pub async fn execute(args: ChatArgs) -> Result<(), CliError> {
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

    // Interactive mode
    if args.interactive {
        return execute_interactive(&client, &args).await;
    }

    // One-shot mode
    let prompt = resolve_prompt(args.prompt, args.file, args.stdin)?;

    let mut messages = Vec::new();
    if let Some(system) = &args.system {
        messages.push(Message::system(system.clone()));
    }
    messages.push(Message::user(prompt));

    let model = args.model.unwrap_or_else(|| client.model().to_string());
    let mut request = ChatCompletionRequest::new(&model, messages);
    if let Some(temp) = args.temperature {
        request = request.temperature(temp);
    }
    if let Some(max) = args.max_tokens {
        request = request.max_tokens(max);
    }

    if args.stream {
        execute_stream(&client, request, args.output).await
    } else {
        execute_blocking(&client, request, args.output).await
    }
}

async fn execute_interactive(client: &AsyncForgeClient, args: &ChatArgs) -> Result<(), CliError> {
    let mut rl = DefaultEditor::new().map_err(|e| CliError::Input(e.to_string()))?;

    let system_prompt = args
        .system
        .clone()
        .unwrap_or_else(|| "You are a helpful assistant.".to_string());
    let model = args
        .model
        .clone()
        .unwrap_or_else(|| client.model().to_string());

    // Conversation history
    let mut messages: Vec<Message> = vec![Message::system(system_prompt)];

    logo::print_compact();
    ui::print_welcome_panel(&[
        format!("{} {}", theme::label("Model"), theme::value(&model)),
        theme::dimmed("Type 'quit' to exit, 'reset' to clear history").to_string(),
    ]);

    loop {
        let readline = rl.readline(&format!("\n{} ", theme::subheader("❯")));
        match readline {
            Ok(line) => {
                let input: &str = line.trim();
                if input.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(input);

                match input.to_lowercase().as_str() {
                    "quit" | "exit" | "q" => break,
                    "reset" => {
                        messages.clear();
                        messages.push(Message::system(
                            args.system
                                .clone()
                                .unwrap_or_else(|| "You are a helpful assistant.".to_string()),
                        ));
                        println!("{}", theme::warning("Conversation reset."));
                        continue;
                    }
                    _ => {}
                }

                // Add user message
                messages.push(Message::user(input.to_string()));

                // Build request
                let mut request = ChatCompletionRequest::new(&model, messages.clone());
                if let Some(temp) = args.temperature {
                    request = request.temperature(temp);
                }
                if let Some(max) = args.max_tokens {
                    request = request.max_tokens(max);
                }

                print!("\n{} ", theme::success("●"));
                std::io::stdout().flush()?;

                // Stream response
                let mut stream = client.chat_completions_stream(request).await?;
                let mut response_content = String::new();

                while let Some(chunk) = stream.next().await {
                    let chunk = chunk?;
                    if let Some(content) = chunk.content() {
                        print!("{}", content);
                        std::io::stdout().flush()?;
                        response_content.push_str(content);
                    }
                }
                println!();

                // Add assistant response to history
                messages.push(Message::assistant(response_content));
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                eprintln!("{} {:?}", theme::error_text("Error:"), err);
                break;
            }
        }
    }

    println!("\nSession ended.");
    Ok(())
}

async fn execute_blocking(
    client: &AsyncForgeClient,
    request: ChatCompletionRequest,
    format: OutputFormat,
) -> Result<(), CliError> {
    let completion = client.chat_completions(request).await?;
    output::print_completion(&completion, format);
    Ok(())
}

async fn execute_stream(
    client: &AsyncForgeClient,
    request: ChatCompletionRequest,
    format: OutputFormat,
) -> Result<(), CliError> {
    let mut stream = client.chat_completions_stream(request).await?;

    match format {
        OutputFormat::Json => {
            let mut chunks = Vec::new();
            while let Some(chunk) = stream.next().await {
                chunks.push(chunk?);
            }
            println!("{}", serde_json::to_string_pretty(&chunks).unwrap());
        }
        _ => {
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                if let Some(content) = chunk.content() {
                    print!("{}", content);
                    std::io::stdout().flush()?;
                }
            }
            println!();
            if matches!(format, OutputFormat::Pretty) {
                eprintln!("{}", theme::dimmed("[stream complete]"));
            }
        }
    }

    Ok(())
}
