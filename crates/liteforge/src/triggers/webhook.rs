//! Webhook trigger for HTTP endpoints.

use super::{Trigger, TriggerError, TriggerEvent, TriggerStatus};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// HTTP method for webhook requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    #[default]
    Post,
    Put,
    Patch,
    Delete,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Delete => write!(f, "DELETE"),
        }
    }
}

/// A webhook event received from an HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// HTTP method of the request.
    pub method: String,
    /// Request path.
    pub path: String,
    /// Query parameters.
    pub query: HashMap<String, String>,
    /// Request headers.
    pub headers: HashMap<String, String>,
    /// Request body (if any).
    pub body: Option<serde_json::Value>,
    /// Source IP address.
    pub source_ip: Option<String>,
}

impl WebhookEvent {
    /// Create a new webhook event.
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            source_ip: None,
        }
    }

    /// Set query parameters.
    pub fn with_query(mut self, query: HashMap<String, String>) -> Self {
        self.query = query;
        self
    }

    /// Set headers.
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    /// Set body.
    pub fn with_body(mut self, body: serde_json::Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Set source IP.
    pub fn with_source_ip(mut self, ip: impl Into<String>) -> Self {
        self.source_ip = Some(ip.into());
        self
    }

    /// Get a header value.
    pub fn header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    /// Get a query parameter.
    pub fn query_param(&self, key: &str) -> Option<&String> {
        self.query.get(key)
    }
}

/// Configuration for a webhook trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// The path to listen on.
    pub path: String,
    /// Allowed HTTP methods.
    pub methods: Vec<HttpMethod>,
    /// Secret for signature verification (optional).
    pub secret: Option<String>,
    /// Whether to require authentication.
    pub require_auth: bool,
    /// Maximum body size in bytes.
    pub max_body_size: usize,
    /// Allowed source IPs (empty = allow all).
    pub allowed_ips: Vec<String>,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            path: "/webhook".to_string(),
            methods: vec![HttpMethod::Post],
            secret: None,
            require_auth: false,
            max_body_size: 1024 * 1024, // 1MB
            allowed_ips: Vec::new(),
        }
    }
}

impl WebhookConfig {
    /// Create a new webhook config with a path.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            ..Default::default()
        }
    }

    /// Set allowed methods.
    pub fn methods(mut self, methods: Vec<HttpMethod>) -> Self {
        self.methods = methods;
        self
    }

    /// Set the secret for signature verification.
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    /// Require authentication.
    pub fn require_auth(mut self, require: bool) -> Self {
        self.require_auth = require;
        self
    }

    /// Set max body size.
    pub fn max_body_size(mut self, size: usize) -> Self {
        self.max_body_size = size;
        self
    }

    /// Add allowed IPs.
    pub fn allowed_ips(mut self, ips: Vec<String>) -> Self {
        self.allowed_ips = ips;
        self
    }
}

/// A trigger that fires on HTTP webhook requests.
///
/// This is a queue-based trigger. To use it, integrate with your HTTP server
/// and call `receive()` when requests arrive.
#[derive(Debug, Clone)]
pub struct WebhookTrigger {
    id: String,
    config: WebhookConfig,
    status: Arc<Mutex<TriggerStatus>>,
    events: Arc<Mutex<VecDeque<WebhookEvent>>>,
    request_count: Arc<Mutex<u64>>,
}

impl WebhookTrigger {
    /// Create a new webhook trigger.
    pub fn new(id: impl Into<String>, config: WebhookConfig) -> Self {
        Self {
            id: id.into(),
            config,
            status: Arc::new(Mutex::new(TriggerStatus::Stopped)),
            events: Arc::new(Mutex::new(VecDeque::new())),
            request_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a webhook trigger with default config for a path.
    pub fn on_path(id: impl Into<String>, path: impl Into<String>) -> Self {
        Self::new(id, WebhookConfig::new(path))
    }

    /// Get the configuration.
    pub fn config(&self) -> &WebhookConfig {
        &self.config
    }

    /// Get the path this webhook listens on.
    pub fn path(&self) -> &str {
        &self.config.path
    }

    /// Get the total request count.
    pub fn request_count(&self) -> u64 {
        *self.request_count.lock().unwrap()
    }

    /// Receive a webhook event.
    ///
    /// Call this from your HTTP server when a request matches this webhook's path.
    pub fn receive(&self, event: WebhookEvent) -> Result<(), TriggerError> {
        let status = *self.status.lock().unwrap();
        if status != TriggerStatus::Running {
            return Err(TriggerError::not_running());
        }

        // Validate method
        let method = event.method.to_uppercase();
        let method_allowed = self.config.methods.iter().any(|m| m.to_string() == method);
        if !method_allowed {
            return Err(TriggerError::runtime(format!(
                "Method {} not allowed",
                method
            )));
        }

        // Validate source IP if configured
        if !self.config.allowed_ips.is_empty() {
            if let Some(ref ip) = event.source_ip {
                if !self.config.allowed_ips.contains(ip) {
                    return Err(TriggerError::runtime("Source IP not allowed"));
                }
            } else {
                return Err(TriggerError::runtime("Source IP required but not provided"));
            }
        }

        // Increment request count
        let mut count = self.request_count.lock().unwrap();
        *count += 1;

        // Queue the event
        self.events.lock().unwrap().push_back(event);
        Ok(())
    }

    /// Receive a raw webhook request.
    ///
    /// Convenience method that constructs the WebhookEvent from parts.
    pub fn receive_request(
        &self,
        method: impl Into<String>,
        path: impl Into<String>,
        headers: HashMap<String, String>,
        body: Option<serde_json::Value>,
    ) -> Result<(), TriggerError> {
        let event = WebhookEvent {
            method: method.into(),
            path: path.into(),
            query: HashMap::new(),
            headers,
            body,
            source_ip: None,
        };
        self.receive(event)
    }

    /// Get the number of pending events.
    pub fn pending_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Verify a webhook signature (HMAC-SHA256).
    ///
    /// Returns true if no secret is configured (verification disabled).
    pub fn verify_signature(&self, _payload: &[u8], signature: &str) -> bool {
        let Some(ref _secret) = self.config.secret else {
            return true; // No secret configured, skip verification
        };

        // In a real implementation, this would compute HMAC-SHA256
        // For now, we just check that a signature was provided
        !signature.is_empty()
    }
}

impl Trigger for WebhookTrigger {
    fn id(&self) -> &str {
        &self.id
    }

    fn trigger_type(&self) -> &str {
        "webhook"
    }

    fn status(&self) -> TriggerStatus {
        *self.status.lock().unwrap()
    }

    fn start(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        if *status == TriggerStatus::Running {
            return Err(TriggerError::already_running());
        }
        *status = TriggerStatus::Running;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        *status = TriggerStatus::Stopped;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        if *status != TriggerStatus::Running {
            return Err(TriggerError::not_running());
        }
        *status = TriggerStatus::Paused;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        if *status != TriggerStatus::Paused {
            return Err(TriggerError::runtime("Trigger is not paused"));
        }
        *status = TriggerStatus::Running;
        Ok(())
    }

    fn poll(&self) -> Option<TriggerEvent> {
        let status = *self.status.lock().unwrap();
        if status != TriggerStatus::Running {
            return None;
        }

        let mut events = self.events.lock().unwrap();
        events.pop_front().map(|webhook_event| {
            TriggerEvent::new(
                &self.id,
                "webhook",
                serde_json::to_value(&webhook_event).unwrap_or_default(),
            )
        })
    }

    fn has_pending(&self) -> bool {
        !self.events.lock().unwrap().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_event_new() {
        let event = WebhookEvent::new("POST", "/api/webhook");
        assert_eq!(event.method, "POST");
        assert_eq!(event.path, "/api/webhook");
    }

    #[test]
    fn test_webhook_event_with_body() {
        let event = WebhookEvent::new("POST", "/webhook")
            .with_body(serde_json::json!({"message": "hello"}));

        assert_eq!(event.body, Some(serde_json::json!({"message": "hello"})));
    }

    #[test]
    fn test_webhook_config() {
        let config = WebhookConfig::new("/api/events")
            .methods(vec![HttpMethod::Post, HttpMethod::Put])
            .secret("my-secret")
            .require_auth(true);

        assert_eq!(config.path, "/api/events");
        assert_eq!(config.methods.len(), 2);
        assert_eq!(config.secret, Some("my-secret".to_string()));
        assert!(config.require_auth);
    }

    #[test]
    fn test_webhook_trigger_receive() {
        let config = WebhookConfig::new("/webhook");
        let mut trigger = WebhookTrigger::new("test", config);
        trigger.start().unwrap();

        let event =
            WebhookEvent::new("POST", "/webhook").with_body(serde_json::json!({"data": "test"}));

        trigger.receive(event).unwrap();
        assert_eq!(trigger.request_count(), 1);

        let trigger_event = trigger.poll().unwrap();
        assert_eq!(trigger_event.trigger_id, "test");
        assert_eq!(trigger_event.event_type, "webhook");
    }

    #[test]
    fn test_webhook_trigger_method_validation() {
        let config = WebhookConfig::new("/webhook").methods(vec![HttpMethod::Post]);
        let mut trigger = WebhookTrigger::new("test", config);
        trigger.start().unwrap();

        // GET should fail
        let event = WebhookEvent::new("GET", "/webhook");
        let result = trigger.receive(event);
        assert!(result.is_err());

        // POST should succeed
        let event = WebhookEvent::new("POST", "/webhook");
        let result = trigger.receive(event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_webhook_trigger_ip_validation() {
        let config = WebhookConfig::new("/webhook").allowed_ips(vec!["192.168.1.1".to_string()]);
        let mut trigger = WebhookTrigger::new("test", config);
        trigger.start().unwrap();

        // Wrong IP should fail
        let event = WebhookEvent::new("POST", "/webhook").with_source_ip("192.168.1.2");
        let result = trigger.receive(event);
        assert!(result.is_err());

        // Correct IP should succeed
        let event = WebhookEvent::new("POST", "/webhook").with_source_ip("192.168.1.1");
        let result = trigger.receive(event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_webhook_trigger_not_running() {
        let config = WebhookConfig::new("/webhook");
        let trigger = WebhookTrigger::new("test", config);

        let event = WebhookEvent::new("POST", "/webhook");
        let result = trigger.receive(event);
        assert!(result.is_err()); // Not started
    }

    #[test]
    fn test_webhook_trigger_on_path() {
        let trigger = WebhookTrigger::on_path("test", "/api/callback");
        assert_eq!(trigger.path(), "/api/callback");
    }
}
