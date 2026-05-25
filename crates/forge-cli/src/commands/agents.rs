use crate::agents::{discover_agents, AgentConfig, ToolConfig, ToolType};
use crate::error::CliError;
use crate::output::OutputFormat;
use crate::forge_config::ForgeCliConfig;
use crate::{logo, theme, ui};
use comfy_table::{presets, Table};
use futures::StreamExt;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::Write;
use std::path::PathBuf;
use liteforge::mcp::{
    CallToolParams, McpServer, McpServerConfig as SdkMcpServerConfig, McpServerManager,
    McpStdioServer, McpTool, ToolResultContent,
};
use liteforge::{
    AsyncForgeClient, ChatCompletionRequest, Message, ForgeConfig, ToolCall, ToolDefinition,
    ToolParameters,
};

#[derive(clap::Args)]
pub struct AgentsArgs {
    #[command(subcommand)]
    pub command: AgentsCommand,
}

#[derive(clap::Subcommand)]
pub enum AgentsCommand {
    /// List available agents
    List {
        /// Directory containing agent configs (defaults to ./agents or ~/.forge/agents)
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "pretty")]
        output: OutputFormat,
    },

    /// Show agent details
    Info {
        /// Agent name or config file path
        name: String,

        /// Directory containing agent configs (defaults to ./agents or ~/.forge/agents)
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "pretty")]
        output: OutputFormat,
    },

    /// Run an interactive agent session
    Run {
        /// Agent name or config file path
        name: String,

        /// Directory containing agent configs (defaults to ./agents or ~/.forge/agents)
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// API key override
        #[arg(long, env = "LITEFORGE_API_KEY")]
        api_key: Option<String>,

        /// Base URL override
        #[arg(long, env = "LITEFORGE_BASE_URL")]
        base_url: Option<String>,
    },
}

pub async fn execute(args: AgentsArgs) -> Result<(), CliError> {
    match args.command {
        AgentsCommand::List { dir, output } => {
            let resolved_dir = resolve_agents_dir(dir.as_ref());
            list_agents(&resolved_dir, output)
        }
        AgentsCommand::Info { name, dir, output } => {
            let resolved_dir = resolve_agents_dir(dir.as_ref());
            show_agent_info(&name, &resolved_dir, output)
        }
        AgentsCommand::Run {
            name,
            dir,
            api_key,
            base_url,
        } => {
            let resolved_dir = resolve_agents_dir(dir.as_ref());
            run_agent(
                &name,
                &resolved_dir,
                api_key.as_deref(),
                base_url.as_deref(),
            )
            .await
        }
    }
}

/// Resolve the agents directory with fallback logic.
///
/// Priority:
/// 1. Explicit --dir flag
/// 2. Local ./agents (if it exists)
/// 3. Config override from ~/.forge/config.toml
/// 4. Default ~/.forge/agents
fn resolve_agents_dir(explicit_dir: Option<&PathBuf>) -> PathBuf {
    // 1. Explicit flag takes precedence
    if let Some(dir) = explicit_dir {
        return dir.clone();
    }

    // 2. Check local ./agents first (project-specific)
    let local = PathBuf::from("agents");
    if local.is_dir() {
        return local;
    }

    // 3. Fall back to config/default
    ForgeCliConfig::load().agents_dir()
}

fn list_agents(dir: &PathBuf, format: OutputFormat) -> Result<(), CliError> {
    let agents = discover_agents(dir).map_err(|e| CliError::Input(e.to_string()))?;

    if agents.is_empty() {
        println!("No agents found in {}", dir.display());
        println!();
        if !dir.exists() {
            println!("{} Directory does not exist.", theme::warning("Note:"));
            println!(
                "  Run {} to set up default directories.",
                theme::value("forge config init")
            );
        } else {
            println!("Create an agent by adding a YAML file to the directory:");
            println!("  {}/my-agent.yaml", dir.display());
        }
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&agents).unwrap());
        }
        OutputFormat::Raw => {
            for agent in &agents {
                println!("{}", agent.name);
            }
        }
        OutputFormat::Pretty => {
            let mut table = Table::new();
            table.load_preset(presets::UTF8_FULL_CONDENSED);
            table.set_header(vec!["Name", "Model", "Tools", "MCP Servers", "Description"]);

            for agent in &agents {
                table.add_row(vec![
                    agent.name.clone(),
                    agent.model.clone().unwrap_or_else(|| "-".to_string()),
                    agent.tool_count().to_string(),
                    agent.mcp_server_count().to_string(),
                    truncate(&agent.description, 40),
                ]);
            }

            println!("{table}");
            println!(
                "\n{} {}",
                theme::success(&agents.len().to_string()),
                theme::dimmed("agents available")
            );
        }
    }

    Ok(())
}

fn show_agent_info(name: &str, dir: &PathBuf, format: OutputFormat) -> Result<(), CliError> {
    let agent = find_agent(name, dir)?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&agent).unwrap());
        }
        OutputFormat::Raw | OutputFormat::Pretty => {
            println!("{}: {}", theme::label("Name"), agent.name);
            println!(
                "{}: {}",
                theme::label("Description"),
                if agent.description.is_empty() {
                    "-"
                } else {
                    &agent.description
                }
            );
            println!(
                "{}: {}",
                theme::label("Model"),
                agent.model.as_deref().unwrap_or("-")
            );

            if let Some(temp) = agent.temperature {
                println!("{}: {}", theme::label("Temperature"), temp);
            }

            if let Some(max) = agent.max_tokens {
                println!("{}: {}", theme::label("Max Tokens"), max);
            }

            if let Some(system) = &agent.system_prompt {
                println!("\n{}", theme::label("System Prompt:"));
                for line in system.lines().take(10) {
                    println!("  {}", line);
                }
                if system.lines().count() > 10 {
                    println!("  ... (truncated)");
                }
            }

            if !agent.tools.is_empty() {
                println!("\n{}", theme::label("Tools:"));
                for tool in &agent.tools {
                    println!(
                        "  {} - {} ({:?})",
                        theme::value(&tool.name),
                        if tool.description.is_empty() {
                            "No description"
                        } else {
                            &tool.description
                        },
                        tool.tool_type
                    );
                }
            }

            if !agent.mcp_servers.is_empty() {
                println!("\n{}", theme::label("MCP Servers:"));
                for server in &agent.mcp_servers {
                    println!("  {} - {}", theme::value(&server.name), server.command);
                }
            }
        }
    }

    Ok(())
}

async fn run_agent(
    name: &str,
    dir: &PathBuf,
    api_key: Option<&str>,
    base_url: Option<&str>,
) -> Result<(), CliError> {
    let agent = find_agent(name, dir)?;

    // Build LiteForge client with agent settings
    let mut config_builder = ForgeConfig::builder();
    if let Some(key) = api_key {
        config_builder = config_builder.api_key(key.to_string());
    }
    if let Some(url) = base_url {
        config_builder = config_builder.base_url(url.to_string());
    }
    if let Some(model) = &agent.model {
        config_builder = config_builder.default_model(model.clone());
    }
    let client = AsyncForgeClient::with_config(config_builder.build());

    // Start MCP servers from agent config
    let mut mcp_manager = McpServerManager::new();
    for server_config in &agent.mcp_servers {
        let sdk_config = SdkMcpServerConfig::stdio(&server_config.name, &server_config.command)
            .with_args(server_config.args.clone())
            .with_env(server_config.env.clone());
        let mut server = McpStdioServer::new(sdk_config);

        print!("Connecting to MCP server '{}'... ", server_config.name);
        std::io::stdout().flush()?;

        match server.connect().await {
            Ok(_) => {
                println!("{}", theme::success("connected"));
                mcp_manager.add_server(Box::new(server));
            }
            Err(e) => {
                println!("{}", theme::error_text("failed"));
                eprintln!(
                    "  {} Failed to connect to MCP server '{}': {}",
                    theme::warning("Warning:"),
                    server_config.name,
                    e
                );
            }
        }
    }

    // Build tool definitions from agent config and MCP servers
    let tool_definitions = build_tool_definitions(&agent.tools, &mcp_manager).await;

    // Initialize conversation
    let system_prompt = agent
        .system_prompt
        .clone()
        .unwrap_or_else(|| "You are a helpful assistant.".to_string());
    let model = agent
        .model
        .clone()
        .unwrap_or_else(|| client.model().to_string());

    let mut messages: Vec<Message> = vec![Message::system(&system_prompt)];

    // Print header
    logo::print_compact();
    ui::print_welcome_panel(
        &[
            format!("{} {}", theme::label("Agent"), theme::value(&agent.name)),
            format!("{} {}", theme::label("Model"), theme::value(&model)),
            if !tool_definitions.is_empty() {
                format!(
                    "{} {} {}",
                    theme::label("Tools"),
                    theme::value(&tool_definitions.len().to_string()),
                    theme::dimmed(&format!(
                        "({})",
                        tool_definitions
                            .iter()
                            .map(|t| t.function.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                )
            } else {
                String::new()
            },
            theme::dimmed("Type 'quit' to exit, 'reset' to clear history").to_string(),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>(),
    );
    println!();

    // Interactive REPL
    let mut rl = DefaultEditor::new().map_err(|e| CliError::Input(e.to_string()))?;

    loop {
        let readline = rl.readline(&format!("\n{} ", theme::subheader("❯")));
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(input);

                match input.to_lowercase().as_str() {
                    "quit" | "exit" | "q" => break,
                    "reset" => {
                        messages.clear();
                        messages.push(Message::system(&system_prompt));
                        println!("{}", theme::warning("Conversation reset."));
                        continue;
                    }
                    _ => {}
                }

                // Add user message
                messages.push(Message::user(input.to_string()));

                // Tool call loop - keep calling LLM until no more tool calls
                loop {
                    let mut request = ChatCompletionRequest::new(&model, messages.clone());
                    if !tool_definitions.is_empty() {
                        request = request.tools(tool_definitions.clone());
                    }
                    if let Some(temp) = agent.temperature {
                        request = request.temperature(temp);
                    }
                    if let Some(max) = agent.max_tokens {
                        request = request.max_tokens(max);
                    }

                    // Ollama `:cloud` models on litellm.poyner.ai emit
                    // tool_calls as plain-text content under streaming.
                    // For those, fall back to non-streaming so tool_calls
                    // come through as structured data.
                    // Ollama cloud models: name ends in either `:cloud`
                    // (e.g. `qwen3.5:cloud`) or `-cloud` (e.g.
                    // `gemma4:31b-cloud`, `gpt-oss:120b-cloud`).
                    let is_ollama_cloud =
                        model.ends_with(":cloud") || model.ends_with("-cloud");
                    let use_streaming = !is_ollama_cloud;

                    let mut response_content = String::new();
                    let mut tool_calls: Vec<ToolCall> = Vec::new();

                    print!("\n{} ", theme::success("●"));
                    std::io::stdout().flush()?;

                    if use_streaming {
                        let mut stream = client.chat_completions_stream(request).await?;
                        while let Some(chunk) = stream.next().await {
                            let chunk = chunk?;

                            if let Some(content) = chunk.content() {
                                print!("{}", content);
                                std::io::stdout().flush()?;
                                response_content.push_str(content);
                            }

                            if let Some(choice) = chunk.choices.first() {
                                if let Some(delta_calls) = &choice.delta.tool_calls {
                                    for delta_call in delta_calls {
                                        let idx = delta_call.index.unwrap_or(0) as usize;
                                        while tool_calls.len() <= idx {
                                            tool_calls.push(ToolCall::new("", "", ""));
                                        }

                                        if !delta_call.id.is_empty() {
                                            tool_calls[idx].id = delta_call.id.clone();
                                        }
                                        if !delta_call.function.name.is_empty() {
                                            tool_calls[idx].function.name =
                                                delta_call.function.name.clone();
                                        }
                                        tool_calls[idx]
                                            .function
                                            .arguments
                                            .push_str(&delta_call.function.arguments);
                                    }
                                }
                            }
                        }
                    } else {
                        let response = client.chat_completions(request).await?;
                        if let Some(choice) = response.choices.into_iter().next() {
                            if let Some(content) = choice.message.content {
                                print!("{}", content);
                                std::io::stdout().flush()?;
                                response_content = content;
                            }
                            if let Some(calls) = choice.message.tool_calls {
                                tool_calls = calls;
                            }
                        }
                    }
                    println!();

                    // Fallback: some models (notably Ollama `:cloud` /
                    // `-cloud` ones) intermittently emit tool calls as
                    // a JSON-in-markdown blob in `content` instead of
                    // structured `tool_calls`. If we got no structured
                    // calls but the content matches that shape, parse
                    // it into a synthetic tool_call so the agent loop
                    // can still execute it.
                    if tool_calls.is_empty() && !response_content.is_empty() {
                        if let Some(parsed) = parse_text_leaked_tool_call(&response_content) {
                            tool_calls.push(parsed);
                            response_content.clear();
                        }
                    }

                    // Filter out empty tool calls
                    tool_calls.retain(|c| !c.function.name.is_empty());

                    // Check for tool calls
                    if tool_calls.is_empty() {
                        messages.push(Message::assistant(response_content));
                        break; // No more tool calls, exit loop
                    }

                    // Add assistant message with tool calls
                    messages.push(Message {
                        role: "assistant".to_string(),
                        content: if response_content.is_empty() {
                            None
                        } else {
                            Some(response_content)
                        },
                        name: None,
                        tool_calls: Some(tool_calls.clone()),
                        tool_call_id: None,
                    });

                    // Execute tool calls
                    for call in &tool_calls {
                        println!(
                            "\n{} {}",
                            theme::dimmed("Calling tool:"),
                            theme::value(&call.function.name)
                        );

                        let result = execute_tool(&agent.tools, &mcp_manager, call).await;
                        let content = match result {
                            Ok(v) => {
                                let text = if v.is_string() {
                                    v.as_str().unwrap_or("").to_string()
                                } else {
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| v.to_string())
                                };
                                println!("{}", theme::dimmed(&text));
                                text
                            }
                            Err(e) => {
                                let err_msg = format!("Error: {}", e);
                                eprintln!("{}", theme::error_text(&err_msg));
                                err_msg
                            }
                        };

                        messages.push(Message::tool(&call.id, content));
                    }
                }
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

    // Cleanup
    mcp_manager.disconnect_all().await;
    println!("\nSession ended.");
    Ok(())
}

/// Build tool definitions from agent config and connected MCP servers.
async fn build_tool_definitions(
    agent_tools: &[ToolConfig],
    manager: &McpServerManager,
) -> Vec<ToolDefinition> {
    let mut definitions = Vec::new();

    // Add tools from agent config (builtin and function types)
    for tool in agent_tools {
        if tool.tool_type != ToolType::Mcp {
            let def = agent_tool_to_definition(tool);
            definitions.push(def);
        }
    }

    // Add tools from MCP servers
    for (server_name, tools) in manager.list_all_tools().await {
        for mcp_tool in tools {
            let def = mcp_tool_to_definition(&mcp_tool, &server_name);
            definitions.push(def);
        }
    }

    definitions
}

/// Try to recover a tool call that a model emitted as JSON-in-content
/// instead of structured `tool_calls` (a quirk of several Ollama-cloud
/// reasoning models). Recognised shapes (with or without ```json fences):
///   {"name": "<tool>", "arguments": { ... }}
///   {"name": "<tool>", "parameters": { ... }}
/// Returns None if the content does not look like a tool call.
fn parse_text_leaked_tool_call(content: &str) -> Option<ToolCall> {
    let mut text = content.trim();
    if let Some(stripped) = text.strip_prefix("```json") {
        text = stripped.trim();
    } else if let Some(stripped) = text.strip_prefix("```") {
        text = stripped.trim();
    }
    if let Some(stripped) = text.strip_suffix("```") {
        text = stripped.trim();
    }

    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let obj = v.as_object()?;
    let name = obj.get("name").and_then(|n| n.as_str())?;
    let args = obj
        .get("arguments")
        .or_else(|| obj.get("parameters"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let args_json = serde_json::to_string(&args).ok()?;

    Some(ToolCall::new(
        format!("call_leaked_{}", uuid_like()),
        name,
        args_json,
    ))
}

/// Cheap unique-ish suffix without pulling in the uuid crate.
fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos)
}

/// Convert an agent-configured tool to a ToolDefinition for the LLM.
fn agent_tool_to_definition(tool: &ToolConfig) -> ToolDefinition {
    let parameters = if let Some(schema) = &tool.parameters {
        // Convert serde_yaml::Value to JSON for parameters
        if let Ok(json) = serde_json::to_value(schema) {
            if let Some(obj) = json.as_object() {
                let properties = obj
                    .get("properties")
                    .and_then(|p| p.as_object())
                    .cloned()
                    .unwrap_or_default();

                let required = obj.get("required").and_then(|r| r.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                });

                ToolParameters {
                    schema_type: "object".to_string(),
                    properties,
                    required,
                }
            } else {
                ToolParameters::default()
            }
        } else {
            ToolParameters::default()
        }
    } else {
        ToolParameters::default()
    };

    ToolDefinition::new(&tool.name)
        .description(&tool.description)
        .parameters(parameters)
}

/// Convert an MCP tool to a ToolDefinition for the LLM.
fn mcp_tool_to_definition(mcp_tool: &McpTool, _server_name: &str) -> ToolDefinition {
    // Convert MCP input_schema to ToolParameters
    let parameters = if let Some(schema) = mcp_tool.input_schema.as_object() {
        let properties = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .cloned()
            .unwrap_or_default();

        let required = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

        ToolParameters {
            schema_type: "object".to_string(),
            properties,
            required,
        }
    } else {
        ToolParameters::default()
    };

    ToolDefinition::new(&mcp_tool.name)
        .description(mcp_tool.description.clone().unwrap_or_default())
        .parameters(parameters)
}

/// Execute a tool call and return the result.
/// Checks agent-configured tools first, then MCP servers.
async fn execute_tool(
    agent_tools: &[ToolConfig],
    manager: &McpServerManager,
    call: &ToolCall,
) -> Result<serde_json::Value, String> {
    // Check if this is an agent-configured tool
    if let Some(tool) = agent_tools.iter().find(|t| t.name == call.function.name) {
        return execute_agent_tool(tool, call).await;
    }

    // Check MCP servers
    for (server_name, tools) in manager.list_all_tools().await {
        if tools.iter().any(|t| t.name == call.function.name) {
            if let Some(server) = manager.get(&server_name) {
                let arguments: Option<std::collections::HashMap<String, serde_json::Value>> =
                    call.function.parse_arguments().ok().and_then(|v| {
                        if v.is_object() {
                            serde_json::from_value(v).ok()
                        } else {
                            None
                        }
                    });

                let params = CallToolParams {
                    name: call.function.name.clone(),
                    arguments,
                };

                match server.call_tool(params).await {
                    Ok(result) => {
                        // Check for error
                        if result.is_error.unwrap_or(false) {
                            let error_text = result
                                .content
                                .iter()
                                .filter_map(|c| match c {
                                    ToolResultContent::Text { text } => Some(text.clone()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            return Err(error_text);
                        }

                        // Convert ToolResultContent to Value
                        if result.content.len() == 1 {
                            match &result.content[0] {
                                ToolResultContent::Text { text } => {
                                    return Ok(serde_json::Value::String(text.clone()));
                                }
                                ToolResultContent::Image { data, mime_type } => {
                                    return Ok(serde_json::json!({
                                        "type": "image",
                                        "data": data,
                                        "mime_type": mime_type
                                    }));
                                }
                                ToolResultContent::Resource { resource, text } => {
                                    return Ok(serde_json::json!({
                                        "type": "resource",
                                        "uri": resource.uri,
                                        "text": text
                                    }));
                                }
                            }
                        } else {
                            let items: Vec<serde_json::Value> = result
                                .content
                                .iter()
                                .map(|c| match c {
                                    ToolResultContent::Text { text } => {
                                        serde_json::json!({"type": "text", "text": text})
                                    }
                                    ToolResultContent::Image { data, mime_type } => {
                                        serde_json::json!({"type": "image", "data": data, "mime_type": mime_type})
                                    }
                                    ToolResultContent::Resource { resource, text } => {
                                        serde_json::json!({"type": "resource", "uri": resource.uri, "text": text})
                                    }
                                })
                                .collect();
                            return Ok(serde_json::Value::Array(items));
                        }
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
        }
    }

    Err(format!("Tool not found: {}", call.function.name))
}

/// Execute an agent-configured tool (builtin or function type).
async fn execute_agent_tool(
    tool: &ToolConfig,
    call: &ToolCall,
) -> Result<serde_json::Value, String> {
    match tool.tool_type {
        ToolType::Builtin => crate::builtin_tools::execute(tool, call).await,
        ToolType::Function => {
            // Function tools would require custom handlers - not implemented
            Err(format!(
                "The '{}' tool is declared as a function type which requires custom implementation. \
                 Function tools are not yet supported in the agent runtime.",
                tool.name
            ))
        }
        ToolType::Mcp => {
            // MCP tools should be handled by MCP servers, not here
            Err(format!(
                "The '{}' tool is declared as MCP type but no MCP server provides it.",
                tool.name
            ))
        }
    }
}

fn find_agent(name: &str, dir: &PathBuf) -> Result<AgentConfig, CliError> {
    // First check if name is a file path
    let path = PathBuf::from(name);
    if path.exists() && path.is_file() {
        return AgentConfig::from_file(&path).map_err(|e| CliError::Input(e.to_string()));
    }

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
