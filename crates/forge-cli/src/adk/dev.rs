use crate::error::CliError;
use crate::serve;
use crate::serve::config::{RoleConfig, ServeConfig};
use crate::serve::state::AppState;
use crate::theme;
use std::path::Path;

pub async fn execute(project_dir: &Path) -> Result<(), CliError> {
    let adk_path = project_dir.join("adk.yaml");
    let config = super::config::AdkConfig::load(&adk_path).map_err(CliError::Config)?;

    println!();
    println!(
        "  {} Dev mode: {}",
        theme::header("ADK"),
        theme::value(&config.name)
    );
    println!("  {} Watching for changes...", theme::dimmed("→"));
    println!();

    let serve_config = adk_to_serve_config(&config);
    let api_key = std::env::var("LITEFORGE_API_KEY").ok();
    let base_url = std::env::var("LITEFORGE_BASE_URL").ok();

    let state = AppState::new(serve_config.clone(), api_key, base_url);

    load_adk_resources(&config, project_dir, &state).await?;
    spawn_agent_mcp_servers(&state).await;

    serve::start_all(state, &serve_config).await
}

/// Spawn every MCP server declared in every loaded agent's `mcp_servers`
/// list and register them on the shared `mcp_manager`. Without this,
/// the agent_chat (user port) and a2a handlers see an empty mcp_manager
/// and cannot invoke any MCP-backed tools, even though the agent YAML
/// declares them.
async fn spawn_agent_mcp_servers(state: &std::sync::Arc<AppState>) {
    use liteforge::mcp::{McpServer, McpServerConfig as SdkMcpServerConfig, McpStdioServer};

    let agents = state.agents.read().await;
    let mut mcp_mgr = state.mcp_manager.write().await;

    for agent in agents.iter() {
        for sc in &agent.mcp_servers {
            let sdk_config = SdkMcpServerConfig::stdio(&sc.name, &sc.command)
                .with_args(sc.args.clone())
                .with_env(sc.env.clone());
            let mut server = McpStdioServer::new(sdk_config);
            print!(
                "  {} Connecting MCP server '{}' ({})...",
                theme::dimmed("→"),
                sc.name,
                agent.name
            );
            use std::io::Write;
            std::io::stdout().flush().ok();
            match server.connect().await {
                Ok(_) => {
                    println!(" {}", theme::success("ok"));
                    mcp_mgr.add_server(Box::new(server));
                }
                Err(e) => {
                    println!(" {} {}", theme::error_text("failed"), e);
                }
            }
        }
    }
}

pub fn adk_to_serve_config(config: &super::config::AdkConfig) -> ServeConfig {
    // FORGE_HOST overrides the default loopback bind so the multi-port
    // server can be reached from outside its container (e.g. K3s pods
    // need 0.0.0.0). Falls back to 127.0.0.1 for local dev safety.
    let host = std::env::var("FORGE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    ServeConfig {
        user: RoleConfig {
            host: host.clone(),
            port: config.ports.user.port,
            enabled: true,
        },
        mcp: RoleConfig {
            host: host.clone(),
            port: config.ports.mcp.port,
            enabled: true,
        },
        tools: RoleConfig {
            host: host.clone(),
            port: config.ports.tools.port,
            enabled: true,
        },
        a2a: RoleConfig {
            host: host.clone(),
            port: config.ports.a2a.port,
            enabled: true,
        },
        knowledge: RoleConfig {
            host: host.clone(),
            port: config.ports.knowledge.port,
            enabled: true,
        },
        skills: RoleConfig {
            host: host.clone(),
            port: config.ports.skills.port,
            enabled: true,
        },
        agents_dir: None,
    }
}

async fn load_adk_resources(
    config: &super::config::AdkConfig,
    project_dir: &Path,
    state: &std::sync::Arc<AppState>,
) -> Result<(), CliError> {
    let tools_dir = project_dir.join("tools");
    if tools_dir.is_dir() {
        let discovered = super::tools_discover::discover_tools(&tools_dir)?;
        if !discovered.is_empty() {
            println!(
                "  {} Loaded {} Python tool(s)",
                theme::success("✓"),
                discovered.len()
            );
            let mut reg = state.tool_registry.write().await;
            for tool in &discovered {
                let tool_clone = tool.clone();
                let fn_tool = liteforge::FnTool::new(
                    &tool.name,
                    &tool.description,
                    tool.parameters.clone(),
                    move |args: serde_json::Value| {
                        super::tools_discover::execute_python_tool(&tool_clone, &args)
                    },
                );
                reg.register(Box::new(fn_tool));
            }
            let executor = liteforge::ToolExecutor::new(reg.clone());
            *state.tool_executor.write().await = executor;
        }
    }

    for src in &config.knowledge.sources {
        let src_path = project_dir.join(&src.path);
        if src_path.is_dir() {
            let mut docs = Vec::new();
            load_knowledge_dir(&src_path, &src.namespace, &mut docs)?;
            if !docs.is_empty() {
                let count = docs.len();
                state
                    .knowledge
                    .upload(docs)
                    .await
                    .map_err(|e| CliError::Input(format!("Knowledge upload failed: {}", e)))?;
                println!(
                    "  {} Loaded {} document(s) from {}",
                    theme::success("✓"),
                    count,
                    src.path
                );
            }
        } else if src_path.is_file() {
            let content = std::fs::read_to_string(&src_path).map_err(CliError::Io)?;
            let id = src_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("doc")
                .to_string();
            let mut doc = liteforge::Document::new(&id, &content);
            if let Some(ns) = &src.namespace {
                doc = doc.namespace(ns);
            }
            state
                .knowledge
                .upload(vec![doc])
                .await
                .map_err(|e| CliError::Input(format!("Knowledge upload failed: {}", e)))?;
            println!("  {} Loaded document: {}", theme::success("✓"), src.path);
        }
    }

    let skills_dir = project_dir.join("skills");
    if skills_dir.is_dir() {
        let mut count = 0;
        for entry in std::fs::read_dir(&skills_dir).map_err(CliError::Io)? {
            let entry = entry.map_err(CliError::Io)?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = std::fs::read_to_string(&path).map_err(CliError::Io)?;
                if let Ok(skill_config) =
                    serde_json::from_str::<liteforge::skills::SkillConfig>(&content)
                {
                    let skill = liteforge::skills::PromptSkill::new(skill_config);
                    state.skill_registry.write().await.register(skill);
                    count += 1;
                }
            }
        }
        if count > 0 {
            println!("  {} Loaded {} skill(s)", theme::success("✓"), count);
        }
    }

    let agents_dir = project_dir.join("agents");
    if agents_dir.is_dir() {
        let agents = crate::agents::discover_agents(&agents_dir).unwrap_or_default();
        let count = agents.len();
        *state.agents.write().await = agents;
        if count > 0 {
            println!("  {} Loaded {} agent(s)", theme::success("✓"), count);
        }
    }

    println!();
    Ok(())
}

fn load_knowledge_dir(
    dir: &Path,
    namespace: &Option<String>,
    docs: &mut Vec<liteforge::Document>,
) -> Result<(), CliError> {
    for entry in std::fs::read_dir(dir).map_err(CliError::Io)? {
        let entry = entry.map_err(CliError::Io)?;
        let path = entry.path();

        if path.is_dir() {
            load_knowledge_dir(&path, namespace, docs)?;
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "md" | "txt" | "json") {
            continue;
        }

        let content = std::fs::read_to_string(&path).map_err(CliError::Io)?;
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("doc")
            .to_string();

        let mut doc = liteforge::Document::new(&id, &content);
        if let Some(ns) = namespace {
            doc = doc.namespace(ns);
        }
        doc = doc.metadata(
            "source",
            serde_json::Value::String(path.display().to_string()),
        );
        docs.push(doc);
    }
    Ok(())
}
