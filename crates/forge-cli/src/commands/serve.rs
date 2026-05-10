use crate::error::CliError;
use crate::serve;
use crate::serve::config::ServeConfig;
use crate::serve::state::AppState;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct ServeArgs {
    #[command(subcommand)]
    pub command: Option<ServeCommand>,

    /// Path to serve.toml config
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// API key override
    #[arg(long, env = "LITEFORGE_API_KEY")]
    pub api_key: Option<String>,

    /// Base URL override
    #[arg(long, env = "LITEFORGE_BASE_URL")]
    pub base_url: Option<String>,

    /// Directory containing agent configs
    #[arg(short, long)]
    pub agents_dir: Option<PathBuf>,

    /// Override user port
    #[arg(long)]
    pub user_port: Option<u16>,

    /// Override MCP port
    #[arg(long)]
    pub mcp_port: Option<u16>,

    /// Override tools port
    #[arg(long)]
    pub tools_port: Option<u16>,

    /// Override A2A port
    #[arg(long)]
    pub a2a_port: Option<u16>,

    /// Override knowledge port
    #[arg(long)]
    pub knowledge_port: Option<u16>,

    /// Override skills port
    #[arg(long)]
    pub skills_port: Option<u16>,
}

#[derive(clap::Subcommand)]
pub enum ServeCommand {
    /// Start all enabled servers
    All,
    /// Start the user-facing API server
    User,
    /// Start the MCP protocol server
    Mcp,
    /// Start the tools REST server
    Tools,
    /// Start the Agent-to-Agent server
    A2a,
    /// Start the knowledge REST server
    Knowledge,
    /// Start the skills REST server
    Skills,
}

pub async fn execute(args: ServeArgs) -> Result<(), CliError> {
    let mut config = if let Some(path) = &args.config {
        if path.exists() {
            let content = std::fs::read_to_string(path).map_err(CliError::Io)?;
            toml::from_str(&content)
                .map_err(|e| CliError::Config(format!("Invalid serve.toml: {}", e)))?
        } else {
            ServeConfig::default()
        }
    } else {
        ServeConfig::load()
    };

    if let Some(dir) = &args.agents_dir {
        config.agents_dir = Some(dir.clone());
    }
    if let Some(p) = args.user_port {
        config.user.port = p;
    }
    if let Some(p) = args.mcp_port {
        config.mcp.port = p;
    }
    if let Some(p) = args.tools_port {
        config.tools.port = p;
    }
    if let Some(p) = args.a2a_port {
        config.a2a.port = p;
    }
    if let Some(p) = args.knowledge_port {
        config.knowledge.port = p;
    }
    if let Some(p) = args.skills_port {
        config.skills.port = p;
    }

    let state = AppState::new(config.clone(), args.api_key.clone(), args.base_url.clone());

    match args.command {
        None | Some(ServeCommand::All) => serve::start_all(state, &config).await,
        Some(ServeCommand::User) => serve::start_single(state, "user", &config).await,
        Some(ServeCommand::Mcp) => serve::start_single(state, "mcp", &config).await,
        Some(ServeCommand::Tools) => serve::start_single(state, "tools", &config).await,
        Some(ServeCommand::A2a) => serve::start_single(state, "a2a", &config).await,
        Some(ServeCommand::Knowledge) => serve::start_single(state, "knowledge", &config).await,
        Some(ServeCommand::Skills) => serve::start_single(state, "skills", &config).await,
    }
}
