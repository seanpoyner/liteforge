use liteforge::mcp::{
    AuthConfig as RustAuthConfig, McpConfig as RustMcpConfig,
    McpServerConfig as RustMcpServerConfig,
};

#[napi(string_enum)]
pub enum TransportType {
    Stdio,
    Sse,
    Http,
}

#[napi]
pub struct McpServerConfig {
    inner: RustMcpServerConfig,
}

#[napi]
impl McpServerConfig {
    #[napi(factory)]
    pub fn stdio(name: String, command: String) -> Self {
        Self {
            inner: RustMcpServerConfig::stdio(name, command),
        }
    }

    #[napi(factory)]
    pub fn sse(name: String, url: String) -> Self {
        Self {
            inner: RustMcpServerConfig::sse(name, url),
        }
    }

    #[napi(factory)]
    pub fn http(name: String, url: String) -> Self {
        Self {
            inner: RustMcpServerConfig::http(name, url),
        }
    }

    #[napi]
    pub fn with_arg(&mut self, arg: String) -> &Self {
        self.inner = self.inner.clone().with_arg(arg);
        self
    }

    #[napi]
    pub fn with_env(&mut self, key: String, value: String) -> &Self {
        self.inner = self.inner.clone().with_env_var(key, value);
        self
    }

    #[napi]
    pub fn with_timeout(&mut self, secs: u32) -> &Self {
        self.inner = self
            .inner
            .clone()
            .with_timeout(std::time::Duration::from_secs(secs as u64));
        self
    }

    #[napi]
    pub fn with_bearer_token(&mut self, token: String) -> &Self {
        self.inner = self
            .inner
            .clone()
            .with_auth(RustAuthConfig::Bearer { token });
        self
    }

    #[napi]
    pub fn with_api_key(&mut self, header: String, key: String) -> &Self {
        self.inner = self
            .inner
            .clone()
            .with_auth(RustAuthConfig::ApiKey { header, key });
        self
    }

    #[napi(getter)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[napi(getter)]
    pub fn transport(&self) -> String {
        format!("{:?}", self.inner.transport)
    }
}

#[napi]
pub struct McpConfig {
    inner: RustMcpConfig,
}

#[napi]
impl McpConfig {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustMcpConfig::new(),
        }
    }

    #[napi]
    pub fn with_server(&mut self, server: &McpServerConfig) -> &Self {
        self.inner = self.inner.clone().with_server(server.inner.clone());
        self
    }

    #[napi]
    pub fn server_names(&self) -> Vec<String> {
        self.inner
            .server_names()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[napi]
    pub fn get_server(&self, name: String) -> Option<serde_json::Value> {
        self.inner.get_server(&name).map(|s| {
            serde_json::json!({
                "name": s.name,
                "transport": format!("{:?}", s.transport),
                "command": s.command,
                "url": s.url,
            })
        })
    }

    #[napi]
    pub fn server_count(&self) -> u32 {
        self.inner.servers.len() as u32
    }
}
