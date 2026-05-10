//! Agent orchestrator for coordinating multiple agents.
//!
//! Provides high-level orchestration of multiple agents with automatic routing.

use super::intent::{IntentRoute, IntentRouter};
use super::session::{get_or_create, SessionStore};
use super::types::{RoutingDecision, Session, SessionMessage};
use crate::agents::{Agent, AgentStep, ToolCallingAgent};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for agents that can be orchestrated.
#[async_trait]
pub trait OrchestratedAgent: Send + Sync {
    /// Get the agent's name.
    fn name(&self) -> &str;

    /// Process a message and return a response.
    async fn process(&self, message: &str) -> Result<String, OrchestratorError>;
}

/// Error types for orchestration.
#[derive(Debug, Clone)]
pub enum OrchestratorError {
    /// No agent available to handle the request.
    NoAgentAvailable,
    /// The specified agent was not found.
    AgentNotFound(String),
    /// Agent execution failed.
    AgentError(String),
    /// Session error.
    SessionError(String),
    /// Routing failed.
    RoutingError(String),
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrchestratorError::NoAgentAvailable => write!(f, "No agent available"),
            OrchestratorError::AgentNotFound(name) => write!(f, "Agent '{}' not found", name),
            OrchestratorError::AgentError(msg) => write!(f, "Agent error: {}", msg),
            OrchestratorError::SessionError(msg) => write!(f, "Session error: {}", msg),
            OrchestratorError::RoutingError(msg) => write!(f, "Routing error: {}", msg),
        }
    }
}

impl std::error::Error for OrchestratorError {}

/// Result of an orchestration request.
#[derive(Debug, Clone)]
pub struct OrchestrationResult {
    /// The agent that handled the request.
    pub agent: String,
    /// The response message.
    pub response: String,
    /// The routing decision that was made.
    pub routing: Option<RoutingDecision>,
    /// Steps taken by the agent.
    pub steps: Vec<AgentStep>,
    /// Session ID.
    pub session_id: String,
}

/// Orchestration strategy for selecting agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrchestrationStrategy {
    /// Route based on intent classification.
    #[default]
    IntentBased,
    /// Round-robin through available agents.
    RoundRobin,
    /// Always use the same agent.
    Fixed,
    /// Let the orchestrator decide based on confidence.
    Adaptive,
}

/// Configuration for the orchestrator.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// The orchestration strategy to use.
    pub strategy: OrchestrationStrategy,
    /// The default agent to use when routing fails.
    pub default_agent: Option<String>,
    /// Minimum confidence for routing decisions.
    pub min_routing_confidence: f32,
    /// Session TTL in seconds.
    pub session_ttl_secs: Option<i64>,
    /// Whether to maintain conversation history.
    pub maintain_history: bool,
    /// Maximum history length.
    pub max_history_length: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            strategy: OrchestrationStrategy::IntentBased,
            default_agent: None,
            min_routing_confidence: 0.5,
            session_ttl_secs: Some(3600),
            maintain_history: true,
            max_history_length: 50,
        }
    }
}

impl OrchestratorConfig {
    /// Create a new orchestrator config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the orchestration strategy.
    pub fn strategy(mut self, strategy: OrchestrationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the default agent.
    pub fn default_agent(mut self, agent: impl Into<String>) -> Self {
        self.default_agent = Some(agent.into());
        self
    }

    /// Set minimum routing confidence.
    pub fn min_routing_confidence(mut self, confidence: f32) -> Self {
        self.min_routing_confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set session TTL.
    pub fn session_ttl_secs(mut self, secs: i64) -> Self {
        self.session_ttl_secs = Some(secs);
        self
    }

    /// Enable or disable conversation history.
    pub fn maintain_history(mut self, maintain: bool) -> Self {
        self.maintain_history = maintain;
        self
    }
}

/// Agent orchestrator for coordinating multiple agents.
pub struct AgentOrchestrator {
    agents: Arc<RwLock<HashMap<String, Arc<dyn OrchestratedAgent>>>>,
    router: Arc<RwLock<IntentRouter>>,
    sessions: SessionStore,
    config: OrchestratorConfig,
    round_robin_index: Arc<RwLock<usize>>,
}

impl AgentOrchestrator {
    /// Create a new agent orchestrator.
    pub fn new(config: OrchestratorConfig) -> Self {
        let mut store = SessionStore::new();
        if let Some(ttl) = config.session_ttl_secs {
            store = store.with_default_ttl_secs(ttl);
        }

        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            router: Arc::new(RwLock::new(
                IntentRouter::new().min_confidence(config.min_routing_confidence),
            )),
            sessions: store,
            config,
            round_robin_index: Arc::new(RwLock::new(0)),
        }
    }

    /// Register an agent with the orchestrator.
    pub async fn register_agent(&self, agent: Arc<dyn OrchestratedAgent>) {
        let name = agent.name().to_string();
        self.agents.write().await.insert(name, agent);
    }

    /// Register an agent with an intent route.
    pub async fn register_agent_with_route(
        &self,
        agent: Arc<dyn OrchestratedAgent>,
        route: IntentRoute,
    ) {
        let name = agent.name().to_string();
        self.agents.write().await.insert(name, agent);
        let mut router = self.router.write().await;
        *router = std::mem::take(&mut *router).route(route);
    }

    /// Set the intent router.
    pub async fn set_router(&self, router: IntentRouter) {
        *self.router.write().await = router;
    }

    /// Process a message with automatic agent selection.
    pub async fn process(
        &self,
        session_id: &str,
        message: &str,
    ) -> Result<OrchestrationResult, OrchestratorError> {
        // Get or create session
        let mut session = get_or_create(&self.sessions, session_id).await;

        // Add user message to history
        if self.config.maintain_history {
            session.add_message(SessionMessage::user(message));
            self.trim_history(&mut session);
        }

        // Select agent based on strategy
        let (agent_name, routing) = self.select_agent(message).await?;

        // Get the agent
        let agents = self.agents.read().await;
        let agent = agents
            .get(&agent_name)
            .ok_or_else(|| OrchestratorError::AgentNotFound(agent_name.clone()))?;

        // Process message
        let response = agent.process(message).await?;

        // Add assistant response to history
        if self.config.maintain_history {
            session.add_message(SessionMessage::assistant(
                &response,
                Some(agent_name.clone()),
            ));
            self.trim_history(&mut session);
        }

        // Save session
        self.sessions.update(session).await;

        Ok(OrchestrationResult {
            agent: agent_name,
            response,
            routing,
            steps: Vec::new(),
            session_id: session_id.to_string(),
        })
    }

    /// Process a message with a specific agent.
    pub async fn process_with_agent(
        &self,
        session_id: &str,
        message: &str,
        agent_name: &str,
    ) -> Result<OrchestrationResult, OrchestratorError> {
        let mut session = get_or_create(&self.sessions, session_id).await;

        if self.config.maintain_history {
            session.add_message(SessionMessage::user(message));
            self.trim_history(&mut session);
        }

        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_name)
            .ok_or_else(|| OrchestratorError::AgentNotFound(agent_name.to_string()))?;

        let response = agent.process(message).await?;

        if self.config.maintain_history {
            session.add_message(SessionMessage::assistant(
                &response,
                Some(agent_name.to_string()),
            ));
            self.trim_history(&mut session);
        }

        self.sessions.update(session).await;

        Ok(OrchestrationResult {
            agent: agent_name.to_string(),
            response,
            routing: None,
            steps: Vec::new(),
            session_id: session_id.to_string(),
        })
    }

    /// Select an agent based on the orchestration strategy.
    async fn select_agent(
        &self,
        message: &str,
    ) -> Result<(String, Option<RoutingDecision>), OrchestratorError> {
        match self.config.strategy {
            OrchestrationStrategy::IntentBased => {
                let router = self.router.read().await;
                if let Some(decision) = router.classify_and_route(message) {
                    if decision.confidence >= self.config.min_routing_confidence {
                        return Ok((decision.agent.clone(), Some(decision)));
                    }
                }

                // Fall back to default
                if let Some(ref default) = self.config.default_agent {
                    Ok((default.clone(), None))
                } else {
                    Err(OrchestratorError::NoAgentAvailable)
                }
            }
            OrchestrationStrategy::RoundRobin => {
                let agents = self.agents.read().await;
                let names: Vec<_> = agents.keys().cloned().collect();

                if names.is_empty() {
                    return Err(OrchestratorError::NoAgentAvailable);
                }

                let mut index = self.round_robin_index.write().await;
                let agent = names[*index % names.len()].clone();
                *index = (*index + 1) % names.len();

                Ok((agent, None))
            }
            OrchestrationStrategy::Fixed => self
                .config
                .default_agent
                .clone()
                .map(|a| (a, None))
                .ok_or(OrchestratorError::NoAgentAvailable),
            OrchestrationStrategy::Adaptive => {
                let router = self.router.read().await;
                if let Some(decision) = router.classify_and_route(message) {
                    // High confidence: use routed agent
                    if decision.confidence >= 0.7 {
                        return Ok((decision.agent.clone(), Some(decision)));
                    }
                    // Medium confidence: prefer routed but could use default
                    if decision.confidence >= 0.4 {
                        return Ok((decision.agent.clone(), Some(decision)));
                    }
                }

                // Low/no confidence: use default
                if let Some(ref default) = self.config.default_agent {
                    Ok((default.clone(), None))
                } else {
                    // Last resort: first available agent
                    let agents = self.agents.read().await;
                    agents
                        .keys()
                        .next()
                        .cloned()
                        .map(|a| (a, None))
                        .ok_or(OrchestratorError::NoAgentAvailable)
                }
            }
        }
    }

    /// Trim history to max length.
    fn trim_history(&self, session: &mut Session) {
        if session.history.len() > self.config.max_history_length {
            let to_remove = session.history.len() - self.config.max_history_length;
            session.history.drain(0..to_remove);
        }
    }

    /// Get a session by ID.
    pub async fn get_session(&self, id: &str) -> Option<Session> {
        self.sessions.get_valid(id).await
    }

    /// List all registered agents.
    pub async fn list_agents(&self) -> Vec<String> {
        self.agents.read().await.keys().cloned().collect()
    }

    /// Get the number of active sessions.
    pub async fn session_count(&self) -> usize {
        self.sessions.count().await
    }

    /// Cleanup expired sessions.
    pub async fn cleanup_sessions(&self) -> usize {
        self.sessions.cleanup_expired().await
    }
}

impl std::fmt::Debug for AgentOrchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentOrchestrator")
            .field("config", &self.config)
            .finish()
    }
}

/// A simple wrapper to make ToolCallingAgent work with the orchestrator.
pub struct ToolCallingAgentWrapper {
    name: String,
    agent: Arc<tokio::sync::Mutex<ToolCallingAgent>>,
}

impl ToolCallingAgentWrapper {
    /// Create a new wrapper.
    pub fn new(name: impl Into<String>, agent: ToolCallingAgent) -> Self {
        Self {
            name: name.into(),
            agent: Arc::new(tokio::sync::Mutex::new(agent)),
        }
    }
}

#[async_trait]
impl OrchestratedAgent for ToolCallingAgentWrapper {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(&self, message: &str) -> Result<String, OrchestratorError> {
        let mut agent = self.agent.lock().await;
        Agent::run(&mut *agent, message)
            .await
            .map_err(|e| OrchestratorError::AgentError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestAgent {
        name: String,
        response: String,
    }

    #[async_trait]
    impl OrchestratedAgent for TestAgent {
        fn name(&self) -> &str {
            &self.name
        }

        async fn process(&self, _message: &str) -> Result<String, OrchestratorError> {
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let config = OrchestratorConfig::new()
            .strategy(OrchestrationStrategy::IntentBased)
            .default_agent("fallback");

        let orchestrator = AgentOrchestrator::new(config);
        assert_eq!(orchestrator.list_agents().await.len(), 0);
    }

    #[tokio::test]
    async fn test_register_agent() {
        let orchestrator = AgentOrchestrator::new(OrchestratorConfig::default());

        let agent = Arc::new(TestAgent {
            name: "test".to_string(),
            response: "Hello!".to_string(),
        });

        orchestrator.register_agent(agent).await;
        assert_eq!(orchestrator.list_agents().await.len(), 1);
    }

    #[tokio::test]
    async fn test_process_with_fixed_strategy() {
        let config = OrchestratorConfig::new()
            .strategy(OrchestrationStrategy::Fixed)
            .default_agent("test");

        let orchestrator = AgentOrchestrator::new(config);

        let agent = Arc::new(TestAgent {
            name: "test".to_string(),
            response: "Hello!".to_string(),
        });

        orchestrator.register_agent(agent).await;

        let result = orchestrator.process("session1", "Hi").await.unwrap();
        assert_eq!(result.agent, "test");
        assert_eq!(result.response, "Hello!");
    }

    #[tokio::test]
    async fn test_process_with_round_robin() {
        let config = OrchestratorConfig::new().strategy(OrchestrationStrategy::RoundRobin);

        let orchestrator = AgentOrchestrator::new(config);

        let agent1 = Arc::new(TestAgent {
            name: "agent1".to_string(),
            response: "Response 1".to_string(),
        });
        let agent2 = Arc::new(TestAgent {
            name: "agent2".to_string(),
            response: "Response 2".to_string(),
        });

        orchestrator.register_agent(agent1).await;
        orchestrator.register_agent(agent2).await;

        // Both agents should be used across multiple calls
        let mut seen = std::collections::HashSet::new();
        for _ in 0..4 {
            let result = orchestrator.process("session1", "Hi").await.unwrap();
            seen.insert(result.agent);
        }

        assert!(seen.len() >= 2); // Both agents should have been used
    }

    #[tokio::test]
    async fn test_session_management() {
        let config = OrchestratorConfig::new()
            .strategy(OrchestrationStrategy::Fixed)
            .default_agent("test")
            .maintain_history(true);

        let orchestrator = AgentOrchestrator::new(config);

        let agent = Arc::new(TestAgent {
            name: "test".to_string(),
            response: "Hello!".to_string(),
        });

        orchestrator.register_agent(agent).await;

        orchestrator.process("session1", "Message 1").await.unwrap();
        orchestrator.process("session1", "Message 2").await.unwrap();

        let session = orchestrator.get_session("session1").await.unwrap();
        assert_eq!(session.history.len(), 4); // 2 user + 2 assistant messages
    }
}
