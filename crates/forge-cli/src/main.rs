#![allow(dead_code)] // CLI utilities often have helper functions for future use

mod adk;
mod agents;
mod builtin_tools;
mod commands;
mod error;
mod input;
mod logo;
mod output;
mod paths;
mod serve;
mod theme;
mod forge_config;
mod ui;
mod usage;

use clap::Parser;

/// Forge CLI - Chat completions, model management, and configuration
///
/// A high-performance command-line interface for the LiteForge,
/// powered by the liteforge Rust SDK.
///
/// Quick start:
///   forge chat "What is the capital of France?"
///   forge chat --stream "Tell me a story"
///   forge chat -i   # interactive mode
///   forge models list
///   forge config show
#[derive(Parser)]
#[command(
    name = "forge",
    version,
    about = "CLI for LiteForge - chat completions, model management, and configuration",
    long_about,
    before_help = logo::help_banner(),
    styles = theme::clap_styles(),
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Chat with an LLM
    ///
    /// Send a prompt to the LLM and get a response.
    /// Supports streaming, system prompts, interactive mode, and model selection.
    ///
    /// Examples:
    ///   forge chat "What is 2+2?"
    ///   forge chat --stream "Tell me a joke"
    ///   forge chat -i                           # interactive REPL
    ///   forge chat --system "You are a pirate" "Where is the treasure?"
    ///   forge chat --model gpt-4o "Explain quantum computing"
    ///   echo "Hello" | forge chat
    Chat(commands::chat::ChatArgs),

    /// Manage available models
    ///
    /// List or inspect models available on the LiteForge.
    ///
    /// Examples:
    ///   forge models list
    ///   forge models info gpt-4o
    Models(commands::models::ModelsArgs),

    /// View and update configuration
    ///
    /// Manage API keys, base URLs, and default settings.
    ///
    /// Examples:
    ///   forge config show
    ///   forge config set api-key sk-xxxx
    ///   forge config get model
    Config(commands::config::ConfigArgs),

    /// Create embeddings from text
    ///
    /// Generate vector embeddings for text input.
    ///
    /// Examples:
    ///   forge embed "Hello world"
    ///   forge embed --file document.txt
    ///   echo "text" | forge embed
    Embed(commands::embed::EmbedArgs),

    /// Chunk text into smaller pieces
    ///
    /// Split text files into chunks for RAG pipelines.
    ///
    /// Examples:
    ///   forge chunk document.txt
    ///   forge chunk document.txt --size 1000 --overlap 100
    ///   forge chunk document.txt --strategy sentence --json
    Chunk(commands::chunk::ChunkArgs),

    /// Manage agents
    ///
    /// List, inspect, and run agents defined in YAML configs.
    ///
    /// Examples:
    ///   forge agents list
    ///   forge agents info my-agent
    ///   forge agents run my-agent
    Agents(commands::agents::AgentsArgs),

    /// Manage tools
    ///
    /// List tools available to agents.
    ///
    /// Examples:
    ///   forge tools list
    ///   forge tools list --agent my-agent
    Tools(commands::tools::ToolsArgs),

    /// Launch Claude Code with LiteForge configuration
    ///
    /// Sets up environment variables and MCP servers for Claude Code.
    /// Optionally tracks usage to a local SQLite database.
    ///
    /// Examples:
    ///   forge claude                    # launch with LiteForge config
    ///   forge claude --print-env        # show env vars
    ///   forge claude --print-mcp        # show MCP config
    ///   forge claude --no-track         # disable usage tracking
    Claude(commands::claude::ClaudeArgs),

    /// View API usage reports
    ///
    /// Show usage statistics from the local tracking database.
    ///
    /// Examples:
    ///   forge usage                     # monthly summary
    ///   forge usage --period weekly     # weekly summary
    ///   forge usage --by-model          # breakdown by model
    ///   forge usage --sessions          # list sessions
    Usage(commands::usage::UsageArgs),

    /// Check text for PII and injection attempts
    ///
    /// Scan text for potential security and privacy issues.
    ///
    /// Examples:
    ///   forge guardrails check "text with email@example.com"
    ///   forge guardrails check --pii "text to check"
    ///   forge guardrails check --injection "ignore previous instructions"
    ///   forge guardrails check --all --file input.txt
    Guardrails(commands::guardrails::GuardrailsArgs),

    /// Manage MCP server configurations
    ///
    /// List and inspect Model Context Protocol servers.
    ///
    /// Examples:
    ///   forge mcp list
    ///   forge mcp info server-name
    ///   forge mcp list --config custom.json
    Mcp(commands::mcp::McpArgs),

    /// Start the multi-port agent server
    ///
    /// Run role-specific servers exposing SDK capabilities.
    ///
    /// Examples:
    ///   forge serve                    # start all servers
    ///   forge serve all                # start all servers
    ///   forge serve user               # user-facing API only
    ///   forge serve mcp                # MCP protocol server only
    ///   forge serve tools              # tools REST server only
    ///   forge serve --user-port 9000   # override user port
    Serve(commands::serve::ServeArgs),

    /// Inspect and validate model-routing configs
    ///
    /// See where a prompt routes and validate router YAML.
    ///
    /// Examples:
    ///   forge route test "what is 2+2?" --router examples/router.yaml
    ///   forge route validate --router examples/router.yaml
    ///   forge route list --router examples/router.yaml
    Route(commands::route::RouteArgs),

    /// Manage infrastructure services
    ///
    /// Start, stop, and monitor observability infrastructure.
    ///
    /// Examples:
    ///   forge infra start -d          # start detached
    ///   forge infra status            # show status
    ///   forge infra logs -f           # follow logs
    ///   forge infra stop              # stop services
    Infra(commands::infra::InfraArgs),

    /// Agent Development Kit
    ///
    /// Scaffold, build, run, and test containerized agent ecosystems.
    ///
    /// Examples:
    ///   forge adk init my-agent       # scaffold a project
    ///   forge adk dev                 # run in dev mode
    ///   forge adk build               # build container
    ///   forge adk run -D              # run container detached
    ///   forge adk test                # run eval suite
    Adk(commands::adk::AdkArgs),
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let raw_args: Vec<String> = std::env::args().collect();
    let cli = Cli::parse_from(preprocess_claude_args(raw_args));

    let result = match cli.command {
        Command::Chat(args) => commands::chat::execute(args).await,
        Command::Models(args) => commands::models::execute(args).await,
        Command::Config(args) => commands::config::execute(args).await,
        Command::Embed(args) => commands::embed::execute(args).await,
        Command::Chunk(args) => commands::chunk::execute(args).await,
        Command::Agents(args) => commands::agents::execute(args).await,
        Command::Tools(args) => commands::tools::execute(args).await,
        Command::Claude(args) => commands::claude::execute(args).await,
        Command::Usage(args) => commands::usage::execute(args).await,
        Command::Guardrails(args) => commands::guardrails::execute(args).await,
        Command::Mcp(args) => commands::mcp::execute(args).await,
        Command::Serve(args) => commands::serve::execute(args).await,
        Command::Route(args) => commands::route::execute(args).await,
        Command::Infra(args) => commands::infra::execute(args).await,
        Command::Adk(args) => commands::adk::execute(args).await,
    };

    if let Err(err) = result {
        eprintln!("{} {}", theme::error_text("error:"), err);
        std::process::exit(err.exit_code());
    }
}

const CLAUDE_KNOWN_OPTIONS: &[&str] = &[
    "--print-env",
    "--print-mcp",
    "--no-track",
    "--api-key",
    "--base-url",
    "--mcp-config",
    "-h",
    "--help",
];

fn preprocess_claude_args(args: Vec<String>) -> Vec<String> {
    let mut iter = args.iter();
    let program = match iter.next() {
        Some(p) => p.clone(),
        None => return args,
    };
    let subcommand = match iter.next() {
        Some(s) => s.clone(),
        None => return args,
    };
    if subcommand != "claude" {
        return args;
    }

    let rest: Vec<String> = iter.cloned().collect();
    if rest.is_empty() || rest.contains(&"--".to_string()) {
        return args;
    }

    let mut result = vec![program, subcommand];
    let mut i = 0;
    let mut found_passthrough = false;
    while i < rest.len() {
        let arg = &rest[i];
        if !found_passthrough && CLAUDE_KNOWN_OPTIONS.contains(&arg.as_str()) {
            result.push(arg.clone());
            if matches!(arg.as_str(), "--api-key" | "--base-url" | "--mcp-config") {
                i += 1;
                if i < rest.len() {
                    result.push(rest[i].clone());
                }
            }
        } else {
            if !found_passthrough {
                result.push("--".to_string());
                found_passthrough = true;
            }
            result.push(arg.clone());
        }
        i += 1;
    }
    result
}
