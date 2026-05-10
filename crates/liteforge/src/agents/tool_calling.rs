//! Tool-calling agent implementation.

use super::context::{AgentContext, AgentState};
use super::step::{AgentStep, StepResult, StepType, TokenUsage};
use super::traits::{Agent, AgentConfig, AgentError, AgentResult};
use crate::client::AsyncForgeClient;
use crate::hooks::{HookContext, HookManager};
use crate::tools::{ToolExecutor, ToolRegistry};
use crate::types::{ChatCompletionRequest, Message, ToolCall, ToolDefinition};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use tracing::Instrument;

/// Maximum bytes of prompt or completion content captured on a span when
/// `LITEFORGE_OTEL_CAPTURE_PROMPTS=true`. Bounded to keep ingest cost predictable.
const PROMPT_CAPTURE_MAX_BYTES: usize = 4096;

/// Truncate a string to at most `max` bytes on a UTF-8 boundary, appending
/// an ellipsis marker if truncated. Used only for span attribute capture.
fn truncate_for_capture(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = s[..end].to_string();
    out.push_str("…[truncated]");
    out
}

/// Read the `LITEFORGE_OTEL_CAPTURE_PROMPTS` env flag. Treated as off unless
/// explicitly enabled so prompt content never leaks to OTel without
/// operator opt-in.
fn capture_prompts_enabled() -> bool {
    std::env::var("LITEFORGE_OTEL_CAPTURE_PROMPTS")
        .ok()
        .map(|s| matches!(s.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// An agent that can call tools to accomplish tasks.
///
/// The ToolCallingAgent follows this loop:
/// 1. Send messages to LLM with available tools
/// 2. If LLM returns tool calls, execute them
/// 3. Add tool results to conversation
/// 4. Repeat until LLM returns a final response or max steps reached
///
/// # Example
///
/// ```no_run
/// use liteforge::agents::{ToolCallingAgent, AgentConfig, Agent};
/// use liteforge::{AsyncForgeClient, ToolRegistry, FnTool};
///
/// #[tokio::main]
/// async fn main() {
///     let client = AsyncForgeClient::new();
///     let mut tools = ToolRegistry::new();
///
///     // Register a simple calculator tool
///     tools.register(Box::new(FnTool::new(
///         "add",
///         "Add two numbers",
///         serde_json::json!({
///             "type": "object",
///             "properties": {
///                 "a": {"type": "number"},
///                 "b": {"type": "number"}
///             },
///             "required": ["a", "b"]
///         }),
///         |args: serde_json::Value| {
///             let a = args["a"].as_f64().unwrap_or(0.0);
///             let b = args["b"].as_f64().unwrap_or(0.0);
///             Ok(serde_json::json!({"result": a + b}))
///         },
///     )));
///
///     let config = AgentConfig::new("calculator-agent")
///         .with_system_prompt("You are a calculator assistant.")
///         .with_tool("add");
///
///     let mut agent = ToolCallingAgent::new(client, tools)
///         .with_config(config);
///
///     let result = agent.run("What is 2 + 3?").await;
///     println!("Result: {:?}", result);
/// }
/// ```
pub struct ToolCallingAgent {
    /// LLM client.
    client: AsyncForgeClient,

    /// Tool executor (contains the registry).
    executor: ToolExecutor,

    /// Agent configuration.
    config: AgentConfig,

    /// Agent context.
    context: AgentContext,

    /// Execution history (steps taken).
    history: Vec<AgentStep>,

    /// Optional hook manager for lifecycle events.
    hooks: Option<Arc<HookManager>>,
}

impl ToolCallingAgent {
    /// Create a new tool-calling agent.
    pub fn new(client: AsyncForgeClient, tools: ToolRegistry) -> Self {
        let executor = ToolExecutor::new(tools);

        Self {
            client,
            executor,
            config: AgentConfig::default(),
            context: AgentContext::default(),
            history: Vec::new(),
            hooks: None,
        }
    }

    /// Set the agent configuration.
    pub fn with_config(mut self, config: AgentConfig) -> Self {
        self.context = AgentContext::new(&config.name)
            .with_max_steps(config.max_steps)
            .with_system_prompt(config.system_prompt.clone().unwrap_or_default());
        self.config = config;
        self
    }

    /// Attach a hook manager. Lifecycle events (`BeforeLlmRequest`,
    /// `AfterLlmResponse`, `BeforeToolCall`, `AfterToolCall`) fire from
    /// `step()`; `BeforeAgentStart`/`BeforeAgentStep`/`AfterAgentStep`/
    /// `AfterAgentEnd` fire from the default `Agent::run()` body.
    ///
    /// `HookResult::Abort` from any pre-event aborts the run with
    /// `AgentError::Other(msg)`. `Skip`/`SkipWith` are treated as `Continue`
    /// for now (deferred, semantics around skipping LLM calls / tool
    /// dispatch need design).
    pub fn with_hooks(mut self, hooks: Arc<HookManager>) -> Self {
        self.hooks = Some(hooks);
        self
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        let prompt = prompt.into();
        self.config.system_prompt = Some(prompt.clone());
        self.context.system_prompt = Some(prompt);
        self
    }

    /// Set the maximum steps.
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.config.max_steps = max_steps;
        self.context.max_steps = max_steps;
        self
    }

    /// Configure the agent to preserve conversation history across resets.
    ///
    /// When enabled, calling `reset()` or finishing a `run()` will not clear
    /// the short-term memory (conversation history). This enables multi-turn
    /// conversations where the agent retains context between `run()` calls.
    ///
    /// # Example
    /// ```ignore
    /// let mut agent = ToolCallingAgent::new(client, tools)
    ///     .with_preserve_history(true);
    ///
    /// // First turn
    /// agent.run("What is 2 + 2?").await?;
    ///
    /// // Second turn - agent remembers the prior conversation
    /// agent.run("And what if we add 3 more?").await?;
    /// ```
    pub fn with_preserve_history(mut self, preserve: bool) -> Self {
        self.context.set_preserve_history(preserve);
        self
    }

    /// Set whether to preserve history across resets (for runtime configuration).
    pub fn set_preserve_history(&mut self, preserve: bool) {
        self.context.set_preserve_history(preserve);
    }

    /// Get the execution history.
    pub fn history(&self) -> &[AgentStep] {
        &self.history
    }

    /// Clear execution history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Get tool definitions for the LLM request.
    fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        if self.config.tools.is_empty() {
            // If no specific tools configured, use all registered tools
            self.executor.registry().definitions()
        } else {
            // Filter to only configured tools
            self.executor
                .registry()
                .definitions()
                .into_iter()
                .filter(|d| self.config.tools.contains(&d.function.name))
                .collect()
        }
    }

    /// Build a chat completion request.
    fn build_request(&self, messages: Vec<Message>) -> ChatCompletionRequest {
        let model = self
            .config
            .model
            .clone()
            .unwrap_or_else(|| self.client.model().to_string());

        let mut request = ChatCompletionRequest::new(model, messages);

        // Add tools
        let tools = self.get_tool_definitions();
        if !tools.is_empty() {
            request = request.tools(tools);
        }

        // Add optional parameters
        if let Some(temp) = self.config.temperature {
            request = request.temperature(temp);
        }
        if let Some(max_tokens) = self.config.max_tokens {
            request = request.max_tokens(max_tokens);
        }

        request
    }

    /// Execute tool calls and return results, firing per-tool hooks.
    /// Each call gets its own `mcp.tool_call` span with name + duration +
    /// result-size attributes.
    async fn execute_tools(&self, tool_calls: &[ToolCall]) -> AgentResult<Vec<Message>> {
        let mut messages = Vec::new();

        for call in tool_calls {
            let tool_span = tracing::info_span!(
                "mcp.tool_call",
                "mcp.tool.name" = %call.function.name,
                "mcp.tool.call_id" = %call.id,
                "mcp.tool.duration_ms" = tracing::field::Empty,
                "mcp.tool.result_size_bytes" = tracing::field::Empty,
            );
            let _tool_guard = tool_span.enter();
            let tool_start = Instant::now();

            // BeforeToolCall, abort halts the agent
            if let Some(h) = &self.hooks {
                let r = h.run(&HookContext::tool_call(
                    &call.function.name,
                    &call.function.arguments,
                ));
                if r.is_abort() {
                    return Err(AgentError::Other(
                        r.error_message().unwrap_or("aborted by hook").to_string(),
                    ));
                }
            }

            let result = self.executor.execute_call(call);
            let content = result.to_message_content();

            tool_span.record(
                "mcp.tool.duration_ms",
                tool_start.elapsed().as_millis() as u64,
            );
            tool_span.record("mcp.tool.result_size_bytes", content.len() as u64);

            // AfterToolCall, best-effort, never aborts
            if let Some(h) = &self.hooks {
                let _ = h.run(&HookContext::tool_result(
                    &call.function.name,
                    serde_json::Value::String(content.clone()),
                ));
            }

            messages.push(Message::tool(&call.id, content));
        }

        Ok(messages)
    }
}

#[async_trait]
impl Agent for ToolCallingAgent {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn config(&self) -> &AgentConfig {
        &self.config
    }

    fn context_mut(&mut self) -> &mut AgentContext {
        &mut self.context
    }

    fn context(&self) -> &AgentContext {
        &self.context
    }

    fn hooks(&self) -> Option<&Arc<HookManager>> {
        self.hooks.as_ref()
    }

    async fn step(&mut self) -> AgentResult<AgentStep> {
        let start = Instant::now();
        let step_number = self.context.current_step;

        // Parent span for this step. Stays open across the LLM + tool work
        // below; child spans (`gen_ai.completion`, `mcp.tool_call`) nest
        // under it.
        let agent_model = self.config.model.as_deref().unwrap_or("");
        let agent_step_span = tracing::info_span!(
            "agent.step",
            "agent.name" = %self.config.name,
            "agent.step.number" = step_number,
            "gen_ai.request.model" = %agent_model,
        );
        let _agent_step_guard = agent_step_span.enter();

        // Update state
        self.context.state = AgentState::Thinking;

        // Build request with current messages
        let messages = self.context.get_messages();
        let request = self.build_request(messages);

        // BeforeLlmRequest, abort halts the agent
        if let Some(h) = &self.hooks {
            let model_str = request.model.clone();
            let msgs_json: Vec<serde_json::Value> = request
                .messages
                .iter()
                .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
                .collect();
            let r = h.run(&HookContext::llm_request(&model_str, &msgs_json));
            if r.is_abort() {
                return Err(AgentError::Other(
                    r.error_message().unwrap_or("aborted by hook").to_string(),
                ));
            }
        }

        // LLM call span, uses OTel GenAI semconv field names. Token
        // counts and finish reasons are recorded after the response lands.
        // `gen_ai.prompts` and `gen_ai.completion.content` are pre-declared
        // so they can be filled in only when capture is enabled, without
        // dynamic field-name lookup (which `tracing::Span::record` silently
        // no-ops on).
        let model_for_span = request.model.clone();
        let temperature_for_span = request.temperature;
        let max_tokens_for_span = request.max_tokens;
        let llm_span = tracing::info_span!(
            "gen_ai.completion",
            "gen_ai.system" = "tipai",
            "gen_ai.request.model" = %model_for_span,
            "gen_ai.request.temperature" = tracing::field::Empty,
            "gen_ai.request.max_tokens" = tracing::field::Empty,
            "gen_ai.request.message_count" = request.messages.len() as u64,
            "gen_ai.usage.input_tokens" = tracing::field::Empty,
            "gen_ai.usage.output_tokens" = tracing::field::Empty,
            "gen_ai.usage.total_tokens" = tracing::field::Empty,
            "gen_ai.response.finish_reasons" = tracing::field::Empty,
            "gen_ai.prompts" = tracing::field::Empty,
            "gen_ai.completion.content" = tracing::field::Empty,
        );
        if let Some(t) = temperature_for_span {
            llm_span.record("gen_ai.request.temperature", t);
        }
        if let Some(m) = max_tokens_for_span {
            llm_span.record("gen_ai.request.max_tokens", m);
        }

        // Optional prompt capture, packs all messages into a single
        // truncated JSON string so we don't depend on dynamic field names.
        if capture_prompts_enabled() {
            let prompts: Vec<serde_json::Value> = request
                .messages
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "role": m.role,
                        "content": m.content.as_deref().unwrap_or(""),
                    })
                })
                .collect();
            let prompts_str = serde_json::to_string(&prompts).unwrap_or_default();
            llm_span.record(
                "gen_ai.prompts",
                truncate_for_capture(&prompts_str, PROMPT_CAPTURE_MAX_BYTES).as_str(),
            );
        }

        // Call LLM under the gen_ai.completion span, `.instrument()` is
        // await-safe, propagates the span across await points.
        let response = self
            .client
            .chat_completions(request)
            .instrument(llm_span.clone())
            .await?;

        // Record response-side attributes on the LLM span.
        if let Some(usage) = response.usage.as_ref() {
            llm_span.record("gen_ai.usage.input_tokens", usage.prompt_tokens);
            llm_span.record("gen_ai.usage.output_tokens", usage.completion_tokens);
            llm_span.record("gen_ai.usage.total_tokens", usage.total_tokens);
        }
        if let Some(first) = response.choices.first() {
            llm_span.record(
                "gen_ai.response.finish_reasons",
                first.finish_reason.as_deref().unwrap_or(""),
            );
            if capture_prompts_enabled() {
                if let Some(content) = first.message.content.as_deref() {
                    llm_span.record(
                        "gen_ai.completion.content",
                        truncate_for_capture(content, PROMPT_CAPTURE_MAX_BYTES).as_str(),
                    );
                }
            }
        }

        // AfterLlmResponse, best-effort, never aborts
        if let Some(h) = &self.hooks {
            let first = response.choices.first();
            let content = first
                .and_then(|c| c.message.content.as_deref())
                .unwrap_or("");
            let tool_call_count = first
                .and_then(|c| c.message.tool_calls.as_ref())
                .map(|tc| tc.len())
                .unwrap_or(0);
            let prompt_tokens = response
                .usage
                .as_ref()
                .map(|u| u.prompt_tokens)
                .unwrap_or(0);
            let completion_tokens = response
                .usage
                .as_ref()
                .map(|u| u.completion_tokens)
                .unwrap_or(0);
            let _ = h.run(&HookContext::llm_response(
                content,
                tool_call_count,
                prompt_tokens,
                completion_tokens,
            ));
        }

        // Extract response
        let choice = response
            .choices
            .first()
            .ok_or_else(|| AgentError::Other("No response from LLM".to_string()))?;

        let assistant_message = &choice.message;

        // Check for tool calls
        if let Some(tool_calls) = &assistant_message.tool_calls {
            if !tool_calls.is_empty() {
                // Add assistant message with tool calls to memory
                self.context.memory.add_message(Message {
                    role: "assistant".to_string(),
                    content: assistant_message.content.clone(),
                    name: None,
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                });

                // Update state
                self.context.state = AgentState::ExecutingTool;

                // Execute tools (fires BeforeToolCall / AfterToolCall hooks)
                let tool_results = self.execute_tools(tool_calls).await?;

                // Add tool results to memory
                for result in &tool_results {
                    self.context.memory.add_message(result.clone());
                }

                // Create step record
                let step = AgentStep::new(
                    step_number,
                    StepType::ToolCall {
                        tool_name: tool_calls
                            .iter()
                            .map(|c| c.function.name.clone())
                            .collect::<Vec<_>>()
                            .join(", "),
                        call_id: tool_calls.first().map(|c| c.id.clone()).unwrap_or_default(),
                    },
                )
                .with_result(StepResult::ToolCalls {
                    calls: tool_calls.clone(),
                })
                .with_duration(start.elapsed())
                .with_tokens(TokenUsage {
                    prompt_tokens: response
                        .usage
                        .as_ref()
                        .map(|u| u.prompt_tokens)
                        .unwrap_or(0),
                    completion_tokens: response
                        .usage
                        .as_ref()
                        .map(|u| u.completion_tokens)
                        .unwrap_or(0),
                    total_tokens: response.usage.as_ref().map(|u| u.total_tokens).unwrap_or(0),
                });

                self.history.push(step.clone());
                return Ok(step);
            }
        }

        // No tool calls - this is a final response
        let response_text = assistant_message
            .content
            .clone()
            .unwrap_or_else(String::new);

        // Add assistant response to memory
        self.context
            .memory
            .add_message(Message::assistant(&response_text));

        // Update state
        self.context.state = AgentState::Completed;

        // Create step record
        let step = AgentStep::new(step_number, StepType::Response)
            .with_result(StepResult::Done {
                response: response_text,
            })
            .with_duration(start.elapsed())
            .with_tokens(TokenUsage {
                prompt_tokens: response
                    .usage
                    .as_ref()
                    .map(|u| u.prompt_tokens)
                    .unwrap_or(0),
                completion_tokens: response
                    .usage
                    .as_ref()
                    .map(|u| u.completion_tokens)
                    .unwrap_or(0),
                total_tokens: response.usage.as_ref().map(|u| u.total_tokens).unwrap_or(0),
            });

        self.history.push(step.clone());
        Ok(step)
    }

    fn reset(&mut self) {
        self.context.reset();
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ForgeConfig;

    fn test_client() -> AsyncForgeClient {
        let config = ForgeConfig::builder().api_key("test-key").build();
        AsyncForgeClient::with_config(config)
    }

    #[test]
    fn test_agent_creation() {
        let client = test_client();
        let tools = ToolRegistry::new();

        let agent = ToolCallingAgent::new(client, tools)
            .with_system_prompt("You are helpful")
            .with_max_steps(5);

        assert_eq!(agent.config().max_steps, 5);
        assert_eq!(
            agent.config().system_prompt,
            Some("You are helpful".to_string())
        );
    }

    #[test]
    fn test_agent_with_config() {
        let client = test_client();
        let tools = ToolRegistry::new();

        let config = AgentConfig::new("my-agent")
            .with_model("gpt-4")
            .with_max_steps(20)
            .with_temperature(0.5);

        let agent = ToolCallingAgent::new(client, tools).with_config(config);

        assert_eq!(agent.name(), "my-agent");
        assert_eq!(agent.config().model, Some("gpt-4".to_string()));
        assert_eq!(agent.context().max_steps, 20);
    }

    #[test]
    fn test_get_tool_definitions_all() {
        let client = test_client();
        let mut tools = ToolRegistry::new();

        tools.register(Box::new(crate::tools::FnTool::new(
            "tool1",
            "Tool 1",
            serde_json::json!({"type": "object"}),
            |_| Ok(serde_json::json!({})),
        )));
        tools.register(Box::new(crate::tools::FnTool::new(
            "tool2",
            "Tool 2",
            serde_json::json!({"type": "object"}),
            |_| Ok(serde_json::json!({})),
        )));

        let agent = ToolCallingAgent::new(client, tools);
        let defs = agent.get_tool_definitions();

        assert_eq!(defs.len(), 2);
    }

    #[test]
    fn test_get_tool_definitions_filtered() {
        let client = test_client();
        let mut tools = ToolRegistry::new();

        tools.register(Box::new(crate::tools::FnTool::new(
            "tool1",
            "Tool 1",
            serde_json::json!({"type": "object"}),
            |_| Ok(serde_json::json!({})),
        )));
        tools.register(Box::new(crate::tools::FnTool::new(
            "tool2",
            "Tool 2",
            serde_json::json!({"type": "object"}),
            |_| Ok(serde_json::json!({})),
        )));

        let config = AgentConfig::new("test").with_tool("tool1");

        let agent = ToolCallingAgent::new(client, tools).with_config(config);
        let defs = agent.get_tool_definitions();

        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].function.name, "tool1");
    }

    #[test]
    fn test_with_preserve_history() {
        let client = test_client();
        let tools = ToolRegistry::new();

        let agent = ToolCallingAgent::new(client, tools).with_preserve_history(true);

        assert!(agent.context().preserve_history);
    }

    #[test]
    fn test_set_preserve_history() {
        let client = test_client();
        let tools = ToolRegistry::new();

        let mut agent = ToolCallingAgent::new(client, tools);
        assert!(!agent.context().preserve_history);

        agent.set_preserve_history(true);
        assert!(agent.context().preserve_history);
    }

    #[test]
    fn test_load_history() {
        let client = test_client();
        let tools = ToolRegistry::new();

        let mut agent = ToolCallingAgent::new(client, tools);

        // Initially empty
        assert_eq!(agent.conversation_messages().len(), 0);

        // Load some history
        let history = vec![
            Message::user("What is 2 + 2?"),
            Message::assistant("The answer is 4."),
        ];
        agent.load_history(history);

        let messages = agent.conversation_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, Some("What is 2 + 2?".to_string()));
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, Some("The answer is 4.".to_string()));
    }

    #[test]
    fn test_reset_clears_history_by_default() {
        let client = test_client();
        let tools = ToolRegistry::new();

        let mut agent = ToolCallingAgent::new(client, tools);
        agent.load_history(vec![Message::user("Hello")]);

        assert_eq!(agent.conversation_messages().len(), 1);

        agent.reset();

        assert_eq!(agent.conversation_messages().len(), 0);
    }

    #[test]
    fn test_reset_preserves_history_when_enabled() {
        let client = test_client();
        let tools = ToolRegistry::new();

        let mut agent = ToolCallingAgent::new(client, tools).with_preserve_history(true);

        agent.load_history(vec![
            Message::user("Hello"),
            Message::assistant("Hi there!"),
        ]);

        assert_eq!(agent.conversation_messages().len(), 2);

        agent.reset();

        // History should be preserved
        assert_eq!(agent.conversation_messages().len(), 2);
    }
}
