use crate::error::CliError;
use crate::output::OutputFormat;
use crate::theme;
use crate::forge_config::ForgeCliConfig;
use comfy_table::{presets, Table};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub command: McpCommand,
}

#[derive(clap::Subcommand)]
pub enum McpCommand {
    /// List configured MCP servers
    List {
        /// MCP config file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "pretty")]
        output: OutputFormat,
    },

    /// Show MCP server details
    Info {
        /// Server name
        name: String,

        /// MCP config file path
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpConfig {
    #[serde(default)]
    mcp_servers: HashMap<String, McpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpServer {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    disabled: bool,
}

pub async fn execute(args: McpArgs) -> Result<(), CliError> {
    match args.command {
        McpCommand::List { config, output } => list_servers(config, output),
        McpCommand::Info { name, config } => show_server_info(&name, config),
    }
}

fn list_servers(config_path: Option<PathBuf>, format: OutputFormat) -> Result<(), CliError> {
    let config = load_config(config_path)?;

    if config.mcp_servers.is_empty() {
        println!("No MCP servers configured.");
        println!("\nAdd servers to an MCP config file:");
        let forge_config = ForgeCliConfig::load();
        println!(
            "  {} (LiteForge)",
            forge_config.mcp_dir().join("servers.json").display()
        );
        println!("  ~/.config/claude/mcp_servers.json (Claude - Linux)");
        println!("  ~/Library/Application Support/Claude/mcp_servers.json (Claude - macOS)");
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&config.mcp_servers).unwrap()
            );
        }
        OutputFormat::Raw => {
            for name in config.mcp_servers.keys() {
                println!("{}", name);
            }
        }
        OutputFormat::Pretty => {
            let mut table = Table::new();
            table.load_preset(presets::UTF8_FULL_CONDENSED);
            table.set_header(vec!["Name", "Command", "Status"]);

            let mut servers: Vec<_> = config.mcp_servers.iter().collect();
            servers.sort_by(|a, b| a.0.cmp(b.0));

            for (name, server) in servers {
                let status = if server.disabled {
                    theme::error_text("disabled")
                } else {
                    theme::success("enabled")
                };

                let cmd = if server.args.is_empty() {
                    server.command.clone()
                } else {
                    format!("{} {}", server.command, server.args.join(" "))
                };

                table.add_row(vec![name.clone(), truncate(&cmd, 50), status]);
            }

            println!("{table}");
            println!(
                "\n{}",
                theme::dimmed(&format!("{} servers configured", config.mcp_servers.len()))
            );
        }
    }

    Ok(())
}

fn show_server_info(name: &str, config_path: Option<PathBuf>) -> Result<(), CliError> {
    let config = load_config(config_path)?;

    let server = config
        .mcp_servers
        .get(name)
        .ok_or_else(|| CliError::Input(format!("Server not found: {}", name)))?;

    println!("{}: {}", theme::label("Name"), name);
    println!("{}: {}", theme::label("Command"), server.command);

    if !server.args.is_empty() {
        println!("{}: {}", theme::label("Args"), server.args.join(" "));
    }

    println!(
        "{}: {}",
        theme::label("Status"),
        if server.disabled {
            theme::error_text("disabled")
        } else {
            theme::success("enabled")
        }
    );

    if !server.env.is_empty() {
        println!("\n{}", theme::label("Environment:"));
        for (key, value) in &server.env {
            let display_value = if key.to_lowercase().contains("key")
                || key.to_lowercase().contains("secret")
                || key.to_lowercase().contains("token")
            {
                mask_value(value)
            } else {
                value.clone()
            };
            println!("  {}={}", key, display_value);
        }
    }

    Ok(())
}

fn load_config(config_path: Option<PathBuf>) -> Result<McpConfig, CliError> {
    let path = config_path.or_else(find_default_config);

    let path = path.ok_or_else(|| {
        CliError::Input("No MCP config file found. Specify one with --config".to_string())
    })?;

    if !path.exists() {
        return Err(CliError::Input(format!(
            "Config file not found: {}",
            path.display()
        )));
    }

    let content = std::fs::read_to_string(&path).map_err(CliError::Io)?;
    let config: McpConfig =
        serde_json::from_str(&content).map_err(|e| CliError::Input(e.to_string()))?;

    Ok(config)
}

fn find_default_config() -> Option<PathBuf> {
    // LiteForge's own MCP config directory (check first)
    let forge_config = ForgeCliConfig::load();
    let forge_mcp = forge_config.mcp_dir().join("servers.json");

    // Try LiteForge config first, then common Claude locations
    let locations = [
        Some(forge_mcp),
        dirs::config_dir().map(|p| p.join("claude").join("mcp_servers.json")),
        dirs::config_dir().map(|p| p.join("claude-code").join("mcp_servers.json")),
        dirs::home_dir().map(|p| p.join(".claude").join("mcp_servers.json")),
        Some(PathBuf::from("mcp_servers.json")),
        Some(PathBuf::from("mcp.json")),
    ];

    locations.into_iter().flatten().find(|loc| loc.exists())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

fn mask_value(s: &str) -> String {
    if s.len() <= 4 {
        "*".repeat(s.len())
    } else {
        format!("{}...{}", &s[..2], &s[s.len() - 2..])
    }
}
