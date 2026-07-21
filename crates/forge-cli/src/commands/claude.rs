use crate::error::CliError;
use crate::theme;
use crate::usage::{ensure_db_dir, RequestRecord, UsageDb};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use liteforge::ForgeConfig;
use uuid::Uuid;

#[derive(clap::Args)]
pub struct ClaudeArgs {
    /// Print environment variables instead of launching
    #[arg(long)]
    pub print_env: bool,

    /// Print MCP server config JSON
    #[arg(long)]
    pub print_mcp: bool,

    /// Disable usage tracking
    #[arg(long)]
    pub no_track: bool,

    /// API key override
    #[arg(long, env = "LITEFORGE_API_KEY")]
    pub api_key: Option<String>,

    /// Base URL override
    #[arg(long, env = "LITEFORGE_BASE_URL")]
    pub base_url: Option<String>,

    /// Additional MCP server config file
    #[arg(long)]
    pub mcp_config: Option<PathBuf>,

    /// Arguments to pass to claude
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
    pub args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpConfig {
    mcp_servers: HashMap<String, McpServer>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpServer {
    command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    args: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    env: HashMap<String, String>,
}

pub async fn execute(args: ClaudeArgs) -> Result<(), CliError> {
    // Load config from env vars and config files (uses correct LiteForge defaults)
    let config = ForgeConfig::from_env();

    // Build environment variables (CLI args override config)
    let upstream_base_url = args.base_url.clone().unwrap_or(config.base_url);
    let api_key = args.api_key.clone().or(config.api_key);

    // Start a local stripping proxy so Claude Code's `context_management`
    // body field and `context-management-*` beta header (which LiteForge's
    // LiteLLM rejects as unknown inputs) are removed before upstream.
    let (proxy_url, _proxy_addr) = super::claude_proxy::start(upstream_base_url.clone())
        .await
        .map_err(CliError::Io)?;

    let base_url = proxy_url;

    let env_vars = build_env_vars(&base_url, api_key.as_deref());

    if args.print_env {
        println!("{}", theme::header("Environment Variables:"));
        for (key, value) in &env_vars {
            if key.contains("KEY") {
                let masked = mask_key(value);
                println!("export {}={}", key, masked);
            } else {
                println!("export {}={}", key, value);
            }
        }
        return Ok(());
    }

    // Build MCP config
    let mcp_config = build_mcp_config(args.mcp_config.as_ref())?;

    if args.print_mcp {
        println!("{}", serde_json::to_string_pretty(&mcp_config).unwrap());
        return Ok(());
    }

    // Find claude binary
    let claude_path = which::which("claude").map_err(|_| {
        CliError::Input(
            "Claude Code not found. Install it from: https://claude.ai/code".to_string(),
        )
    })?;

    // Initialize usage tracking
    let session_id = Uuid::new_v4().to_string();
    let db = if !args.no_track {
        let db_path = ensure_db_dir().map_err(CliError::Io)?;
        let db = UsageDb::open(&db_path).map_err(|e| CliError::Input(e.to_string()))?;
        db.start_session(&session_id, "claude-code")
            .map_err(|e| CliError::Input(e.to_string()))?;
        Some(db)
    } else {
        None
    };

    println!("{}", theme::header("LiteForge Claude Launcher"));
    println!("Upstream: {}", theme::value(&upstream_base_url));
    println!("Proxy:    {}", theme::dimmed(&base_url));
    if let Some(key) = &api_key {
        println!("API Key: {}", theme::value(&mask_key(key)));
    }
    if !args.no_track {
        println!("Session: {}", theme::dimmed(&session_id));
    }
    println!();

    // Launch claude with environment
    let mut cmd = Command::new(&claude_path);
    cmd.envs(&env_vars);

    // Write MCP config to temp file if needed
    if !mcp_config.mcp_servers.is_empty() {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("forge");
        std::fs::create_dir_all(&config_dir).map_err(CliError::Io)?;

        let mcp_path = config_dir.join("mcp-servers.json");
        std::fs::write(
            &mcp_path,
            serde_json::to_string_pretty(&mcp_config).unwrap(),
        )
        .map_err(CliError::Io)?;

        cmd.env("CLAUDE_MCP_CONFIG", &mcp_path);
    }

    // Add any extra args
    cmd.args(&args.args);

    let start = Utc::now();
    let status = cmd.status().map_err(CliError::Io)?;
    let duration = Utc::now() - start;

    // End session and record summary
    if let Some(db) = db {
        db.end_session(&session_id)
            .map_err(|e| CliError::Input(e.to_string()))?;

        // Record a summary request (actual request tracking would need a proxy)
        let summary = RequestRecord {
            session_id: session_id.clone(),
            timestamp: Utc::now(),
            model: "session-summary".to_string(),
            endpoint: "/session".to_string(),
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            latency_ms: Some(duration.num_milliseconds() as u32),
            status_code: Some(if status.success() { 0 } else { 1 }),
            error: None,
        };
        let _ = db.record_request(&summary);

        println!();
        println!("{}", theme::dimmed("Session ended."));
        println!(
            "Duration: {}",
            theme::dimmed(&format_duration(duration.num_seconds()))
        );
        println!("View usage: {}", theme::value("forge usage --sessions"));
    }

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn build_env_vars(base_url: &str, api_key: Option<&str>) -> HashMap<String, String> {
    let mut env = HashMap::new();

    // Set the base URL for various clients
    env.insert("ANTHROPIC_BASE_URL".to_string(), base_url.to_string());
    env.insert("OPENAI_BASE_URL".to_string(), base_url.to_string());
    env.insert("LITEFORGE_BASE_URL".to_string(), base_url.to_string());

    // Set API key. We set ANTHROPIC_AUTH_TOKEN in addition to ANTHROPIC_API_KEY
    // because Claude Code prefers ANTHROPIC_AUTH_TOKEN (sent as
    // `Authorization: Bearer`) over ANTHROPIC_API_KEY (`x-api-key`) and over a
    // cached OAuth login. If a shell exports ANTHROPIC_AUTH_TOKEN for another
    // purpose (e.g. a local Ollama proxy that uses `ollama` as its token), it
    // would leak through here — Claude Code would send that token as
    // `Authorization: Bearer` to the proxy, which forwards it to the upstream
    // gateway, which rejects it with 401 "Invalid proxy server token passed".
    // Forcing ANTHROPIC_AUTH_TOKEN to the real gateway key prevents that leak
    // and also keeps Claude Code from using an OAuth (Claude Max) token here.
    if let Some(key) = api_key {
        env.insert("ANTHROPIC_API_KEY".to_string(), key.to_string());
        env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), key.to_string());
        env.insert("OPENAI_API_KEY".to_string(), key.to_string());
        env.insert("LITEFORGE_API_KEY".to_string(), key.to_string());
    }

    // Map Claude Code's default model names to what LiteForge's LiteLLM exposes.
    // Users can override by exporting ANTHROPIC_MODEL / ANTHROPIC_SMALL_FAST_MODEL
    // themselves before running `forge claude`.
    if std::env::var_os("ANTHROPIC_MODEL").is_none() {
        env.insert(
            "ANTHROPIC_MODEL".to_string(),
            "claude-sonnet-4.6".to_string(),
        );
    }
    if std::env::var_os("ANTHROPIC_SMALL_FAST_MODEL").is_none() {
        env.insert(
            "ANTHROPIC_SMALL_FAST_MODEL".to_string(),
            "claude-haiku-4.5".to_string(),
        );
    }

    // Claude Code's default API request timeout (~60s) is too short for the
    // always-thinking models LiteForge routes to (e.g. kimi-k3, which can spend
    // 60-120s in reasoning before the first content token on complex prompts),
    // so requests time out and Claude Code retries in a loop. Give a generous
    // 5-min ceiling; overridable by exporting API_TIMEOUT_MS before `forge claude`.
    if std::env::var_os("API_TIMEOUT_MS").is_none() {
        env.insert("API_TIMEOUT_MS".to_string(), "300000".to_string());
    }

    env
}

fn build_mcp_config(extra_config: Option<&PathBuf>) -> Result<McpConfig, CliError> {
    let mut config = McpConfig {
        mcp_servers: HashMap::new(),
    };

    // Load extra config if provided
    if let Some(path) = extra_config {
        let content = std::fs::read_to_string(path).map_err(CliError::Io)?;
        let extra: McpConfig =
            serde_json::from_str(&content).map_err(|e| CliError::Input(e.to_string()))?;
        config.mcp_servers.extend(extra.mcp_servers);
    }

    Ok(config)
}

fn mask_key(key: &str) -> String {
    if key.len() > 8 {
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    } else {
        "****".to_string()
    }
}

fn format_duration(secs: i64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
