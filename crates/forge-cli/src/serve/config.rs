use crate::paths;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    #[serde(default = "default_host")]
    pub host: String,
    pub port: u16,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_true() -> bool {
    true
}

impl RoleConfig {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServeConfig {
    #[serde(default = "default_user")]
    pub user: RoleConfig,
    #[serde(default = "default_mcp")]
    pub mcp: RoleConfig,
    #[serde(default = "default_tools")]
    pub tools: RoleConfig,
    #[serde(default = "default_a2a")]
    pub a2a: RoleConfig,
    #[serde(default = "default_knowledge")]
    pub knowledge: RoleConfig,
    #[serde(default = "default_skills")]
    pub skills: RoleConfig,
    #[serde(default)]
    pub agents_dir: Option<PathBuf>,
}

fn default_user() -> RoleConfig {
    RoleConfig {
        host: default_host(),
        port: 8080,
        enabled: true,
    }
}
fn default_mcp() -> RoleConfig {
    RoleConfig {
        host: default_host(),
        port: 8081,
        enabled: true,
    }
}
fn default_tools() -> RoleConfig {
    RoleConfig {
        host: default_host(),
        port: 8082,
        enabled: true,
    }
}
fn default_a2a() -> RoleConfig {
    RoleConfig {
        host: default_host(),
        port: 8083,
        enabled: true,
    }
}
fn default_knowledge() -> RoleConfig {
    RoleConfig {
        host: default_host(),
        port: 8084,
        enabled: true,
    }
}
fn default_skills() -> RoleConfig {
    RoleConfig {
        host: default_host(),
        port: 8085,
        enabled: true,
    }
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            user: default_user(),
            mcp: default_mcp(),
            tools: default_tools(),
            a2a: default_a2a(),
            knowledge: default_knowledge(),
            skills: default_skills(),
            agents_dir: None,
        }
    }
}

impl ServeConfig {
    pub fn load() -> Self {
        let path = paths::forge_config_dir().join("serve.toml");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = toml::from_str(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn agents_dir(&self) -> PathBuf {
        self.agents_dir
            .clone()
            .unwrap_or_else(paths::default_agents_dir)
    }
}
