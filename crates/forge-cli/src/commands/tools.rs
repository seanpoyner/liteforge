use crate::agents::{discover_agents, AgentConfig, ToolType};
use crate::error::CliError;
use crate::output::OutputFormat;
use crate::theme;
use crate::forge_config::ForgeCliConfig;
use comfy_table::{presets, Table};
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct ToolsArgs {
    #[command(subcommand)]
    pub command: ToolsCommand,
}

#[derive(clap::Subcommand)]
pub enum ToolsCommand {
    /// List available tools
    List {
        /// Agent name to list tools for
        #[arg(short, long)]
        agent: Option<String>,

        /// Directory containing agent configs (defaults to ./agents or ~/.forge/agents)
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "pretty")]
        output: OutputFormat,
    },
}

pub async fn execute(args: ToolsArgs) -> Result<(), CliError> {
    match args.command {
        ToolsCommand::List { agent, dir, output } => {
            let resolved_dir = resolve_agents_dir(dir.as_ref());
            list_tools(agent.as_deref(), &resolved_dir, output)
        }
    }
}

/// Resolve the agents directory with fallback logic.
/// Tools are defined within agent configs, so this uses the agents directory.
fn resolve_agents_dir(explicit_dir: Option<&PathBuf>) -> PathBuf {
    if let Some(dir) = explicit_dir {
        return dir.clone();
    }

    let local = PathBuf::from("agents");
    if local.is_dir() {
        return local;
    }

    ForgeCliConfig::load().agents_dir()
}

fn list_tools(
    agent_name: Option<&str>,
    dir: &PathBuf,
    format: OutputFormat,
) -> Result<(), CliError> {
    let agents: Vec<AgentConfig> = if let Some(name) = agent_name {
        // Find specific agent
        let agent = find_agent(name, dir)?;
        vec![agent]
    } else {
        // Get all agents
        discover_agents(dir).map_err(|e| CliError::Input(e.to_string()))?
    };

    // Collect all tools with their agent source
    let mut tools: Vec<(&str, &crate::agents::ToolConfig)> = Vec::new();
    for agent in &agents {
        for tool in &agent.tools {
            tools.push((&agent.name, tool));
        }
    }

    if tools.is_empty() {
        if agent_name.is_some() {
            println!("No tools defined for agent");
        } else {
            println!("No tools found in any agent configs");
        }
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            #[derive(serde::Serialize)]
            struct ToolOutput<'a> {
                agent: &'a str,
                name: &'a str,
                description: &'a str,
                tool_type: ToolType,
            }

            let output: Vec<ToolOutput> = tools
                .iter()
                .map(|(agent, tool)| ToolOutput {
                    agent,
                    name: &tool.name,
                    description: &tool.description,
                    tool_type: tool.tool_type,
                })
                .collect();

            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Raw => {
            for (agent, tool) in &tools {
                println!("{}/{}", agent, tool.name);
            }
        }
        OutputFormat::Pretty => {
            let mut table = Table::new();
            table.load_preset(presets::UTF8_FULL_CONDENSED);

            if agent_name.is_some() {
                table.set_header(vec!["Tool", "Type", "Description"]);
                for (_, tool) in &tools {
                    table.add_row(vec![
                        tool.name.clone(),
                        format!("{:?}", tool.tool_type),
                        truncate(&tool.description, 50),
                    ]);
                }
            } else {
                table.set_header(vec!["Agent", "Tool", "Type", "Description"]);
                for (agent, tool) in &tools {
                    table.add_row(vec![
                        agent.to_string(),
                        tool.name.clone(),
                        format!("{:?}", tool.tool_type),
                        truncate(&tool.description, 40),
                    ]);
                }
            }

            println!("{table}");
            println!(
                "\n{}",
                theme::dimmed(&format!("{} tools available", tools.len()))
            );
        }
    }

    Ok(())
}

fn find_agent(name: &str, dir: &PathBuf) -> Result<AgentConfig, CliError> {
    // Check for name.yaml or name.yml in the directory
    let yaml_path = dir.join(format!("{}.yaml", name));
    if yaml_path.exists() {
        return AgentConfig::from_file(&yaml_path).map_err(|e| CliError::Input(e.to_string()));
    }

    let yml_path = dir.join(format!("{}.yml", name));
    if yml_path.exists() {
        return AgentConfig::from_file(&yml_path).map_err(|e| CliError::Input(e.to_string()));
    }

    // Search all agents in directory
    let agents = discover_agents(dir).map_err(|e| CliError::Input(e.to_string()))?;
    agents
        .into_iter()
        .find(|a| a.name == name)
        .ok_or_else(|| CliError::Input(format!("Agent not found: {}", name)))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
