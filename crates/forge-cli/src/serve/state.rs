use crate::agents::config::AgentConfig as CliAgentConfig;
use crate::agents::discover_agents;
use std::path::PathBuf;
use std::sync::Arc;
use liteforge::knowledge::LocalKnowledgeBackend;
use liteforge::mcp::McpServerManager;
use liteforge::skills::SkillRegistry;
use liteforge::tools::{ToolExecutor, ToolRegistry};
use liteforge::{AsyncForgeClient, KnowledgeClient, ForgeConfig};
use tokio::sync::RwLock;

use super::config::ServeConfig;

pub struct AppState {
    pub client: AsyncForgeClient,
    pub tool_registry: RwLock<ToolRegistry>,
    pub tool_executor: RwLock<ToolExecutor>,
    pub skill_registry: RwLock<SkillRegistry>,
    pub knowledge: Arc<dyn KnowledgeClient>,
    pub mcp_manager: RwLock<McpServerManager>,
    pub agents: RwLock<Vec<CliAgentConfig>>,
    pub agents_dir: PathBuf,
    pub config: ServeConfig,
}

impl AppState {
    pub fn new(
        serve_config: ServeConfig,
        api_key: Option<String>,
        base_url: Option<String>,
    ) -> Arc<Self> {
        let mut builder = ForgeConfig::builder();
        if let Some(key) = api_key {
            builder = builder.api_key(key);
        }
        if let Some(url) = base_url {
            builder = builder.base_url(url);
        }
        let forge_config = builder.build();
        let client = AsyncForgeClient::with_config(forge_config);

        let tool_registry = ToolRegistry::new();
        let tool_executor = ToolExecutor::new(tool_registry.clone());
        let skill_registry = SkillRegistry::new();
        let knowledge: Arc<dyn KnowledgeClient> = Arc::new(LocalKnowledgeBackend::new());
        let mcp_manager = McpServerManager::new();

        let agents_dir = serve_config.agents_dir();
        let agents = discover_agents(&agents_dir).unwrap_or_default();

        Arc::new(Self {
            client,
            tool_registry: RwLock::new(tool_registry),
            tool_executor: RwLock::new(tool_executor),
            skill_registry: RwLock::new(skill_registry),
            knowledge,
            mcp_manager: RwLock::new(mcp_manager),
            agents: RwLock::new(agents),
            agents_dir,
            config: serve_config,
        })
    }
}
