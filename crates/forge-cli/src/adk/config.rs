use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdkConfig {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub ports: PortsConfig,
    #[serde(default)]
    pub secrets: SecretsConfig,
    #[serde(default)]
    pub knowledge: KnowledgeConfig,
    #[serde(default)]
    pub guardrails: GuardrailsConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub persistence: PersistenceConfig,
    #[serde(default)]
    pub hitl: HitlConfig,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl AdkConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read adk.yaml: {}", e))?;
        serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse adk.yaml: {}", e))
    }

    pub fn project_dir(path: impl AsRef<Path>) -> PathBuf {
        path.as_ref()
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_model() -> String {
    "claude-haiku-4.5".to_string()
}
fn default_temperature() -> f32 {
    0.7
}
fn default_max_tokens() -> u32 {
    4096
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfig {
    #[serde(default = "default_port_8080")]
    pub port: u16,
    #[serde(default = "default_true")]
    pub exposed: bool,
}

fn default_port_8080() -> u16 {
    8080
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortsConfig {
    #[serde(default = "default_user_port")]
    pub user: PortConfig,
    #[serde(default = "default_mcp_port")]
    pub mcp: PortConfig,
    #[serde(default = "default_tools_port")]
    pub tools: PortConfig,
    #[serde(default = "default_a2a_port")]
    pub a2a: PortConfig,
    #[serde(default = "default_knowledge_port")]
    pub knowledge: PortConfig,
    #[serde(default = "default_skills_port")]
    pub skills: PortConfig,
}

fn default_user_port() -> PortConfig {
    PortConfig {
        port: 9080,
        exposed: true,
    }
}
fn default_mcp_port() -> PortConfig {
    PortConfig {
        port: 9081,
        exposed: false,
    }
}
fn default_tools_port() -> PortConfig {
    PortConfig {
        port: 9082,
        exposed: false,
    }
}
fn default_a2a_port() -> PortConfig {
    PortConfig {
        port: 9083,
        exposed: true,
    }
}
fn default_knowledge_port() -> PortConfig {
    PortConfig {
        port: 9084,
        exposed: false,
    }
}
fn default_skills_port() -> PortConfig {
    PortConfig {
        port: 9085,
        exposed: false,
    }
}

impl Default for PortsConfig {
    fn default() -> Self {
        Self {
            user: default_user_port(),
            mcp: default_mcp_port(),
            tools: default_tools_port(),
            a2a: default_a2a_port(),
            knowledge: default_knowledge_port(),
            skills: default_skills_port(),
        }
    }
}

impl PortsConfig {
    pub fn exposed_ports(&self) -> Vec<u16> {
        let mut ports = Vec::new();
        if self.user.exposed {
            ports.push(self.user.port);
        }
        if self.mcp.exposed {
            ports.push(self.mcp.port);
        }
        if self.tools.exposed {
            ports.push(self.tools.port);
        }
        if self.a2a.exposed {
            ports.push(self.a2a.port);
        }
        if self.knowledge.exposed {
            ports.push(self.knowledge.port);
        }
        if self.skills.exposed {
            ports.push(self.skills.port);
        }
        ports
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    #[serde(default = "default_secrets_provider")]
    pub provider: String,
    pub aws_region: Option<String>,
    pub aws_secret_name: Option<String>,
}

fn default_secrets_provider() -> String {
    "env".to_string()
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            provider: default_secrets_provider(),
            aws_region: None,
            aws_secret_name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSource {
    pub path: String,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub namespace: Option<String>,
}

fn default_format() -> String {
    "auto".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KnowledgeConfig {
    #[serde(default)]
    pub sources: Vec<KnowledgeSource>,
    pub embedding_model: Option<String>,
    #[serde(default = "default_false")]
    pub auto_embed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPattern {
    pub name: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuardrailsConfig {
    #[serde(default = "default_false")]
    pub pii_detection: bool,
    #[serde(default = "default_false")]
    pub injection_detection: bool,
    #[serde(default)]
    pub custom_patterns: Vec<CustomPattern>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub otlp_endpoint: Option<String>,
    #[serde(default)]
    pub service_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    #[serde(default = "default_false")]
    pub enabled: bool,
    #[serde(default = "default_data_path")]
    pub path: String,
}

fn default_data_path() -> String {
    "/data".to_string()
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: default_data_path(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitlConfig {
    #[serde(default = "default_false")]
    pub enabled: bool,
    pub webhook_url: Option<String>,
    #[serde(default = "default_hitl_timeout")]
    pub timeout_secs: u64,
}

fn default_hitl_timeout() -> u64 {
    300
}

impl Default for HitlConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            webhook_url: None,
            timeout_secs: default_hitl_timeout(),
        }
    }
}
