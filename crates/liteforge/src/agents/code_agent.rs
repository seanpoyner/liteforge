//! Code execution agent.

use super::context::{AgentContext, AgentState};
use super::sandbox::{ExecutionResult, Language, ProcessSandbox, Sandbox, SandboxConfig};
use super::step::{AgentStep, StepResult, StepType, TokenUsage};
use super::traits::{Agent, AgentConfig, AgentError, AgentResult};
use crate::client::AsyncForgeClient;
use crate::types::{ChatCompletionRequest, Message, ToolCall, ToolDefinition, ToolParameters};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Configuration for the code agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAgentConfig {
    /// Base agent configuration.
    #[serde(flatten)]
    pub agent: AgentConfig,
    /// Sandbox configuration.
    #[serde(default)]
    pub sandbox: SandboxConfig,
    /// Default language for code execution.
    #[serde(default)]
    pub default_language: Option<Language>,
    /// Whether to auto-detect language.
    #[serde(default = "default_true")]
    pub auto_detect_language: bool,
    /// Maximum code length.
    #[serde(default = "default_max_code_length")]
    pub max_code_length: usize,
}

fn default_true() -> bool {
    true
}

fn default_max_code_length() -> usize {
    100_000
}

impl Default for CodeAgentConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig::new("code-agent").with_system_prompt(DEFAULT_SYSTEM_PROMPT),
            sandbox: SandboxConfig::default(),
            default_language: Some(Language::Python),
            auto_detect_language: true,
            max_code_length: default_max_code_length(),
        }
    }
}

impl CodeAgentConfig {
    /// Create a new code agent config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.agent.system_prompt = Some(prompt.into());
        self
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.agent.model = Some(model.into());
        self
    }

    /// Set sandbox configuration.
    pub fn with_sandbox(mut self, config: SandboxConfig) -> Self {
        self.sandbox = config;
        self
    }

    /// Set default language.
    pub fn with_default_language(mut self, language: Language) -> Self {
        self.default_language = Some(language);
        self
    }
}

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a code execution assistant. When asked to write and run code:

1. Analyze the request and determine the best approach
2. Write clean, well-documented code
3. Use the execute_code tool to run the code
4. Analyze the output and provide a clear explanation

Available languages: Python, JavaScript, Shell, Ruby

Guidelines:
- Write safe, efficient code
- Handle errors gracefully
- Explain what the code does and its output
- If the code fails, analyze the error and try to fix it
"#;

/// An agent that can write and execute code.
pub struct CodeAgent {
    client: AsyncForgeClient,
    config: CodeAgentConfig,
    context: AgentContext,
    sandbox: Arc<dyn Sandbox>,
    execution_history: Vec<CodeExecution>,
}

/// Record of a code execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecution {
    /// The code that was executed.
    pub code: String,
    /// The language used.
    pub language: Language,
    /// The execution result.
    pub result: ExecutionResult,
}

impl CodeAgent {
    /// Create a new code agent.
    pub fn new(client: AsyncForgeClient) -> Self {
        Self::with_config(client, CodeAgentConfig::default())
    }

    /// Create a code agent with configuration.
    pub fn with_config(client: AsyncForgeClient, config: CodeAgentConfig) -> Self {
        let name = config.agent.name.clone();
        let max_steps = config.agent.max_steps;
        let mut context = AgentContext::new(&name);
        context.max_steps = max_steps;
        Self {
            client,
            config,
            context,
            sandbox: Arc::new(ProcessSandbox::new()),
            execution_history: Vec::new(),
        }
    }

    /// Create a code agent with a custom sandbox.
    pub fn with_sandbox<S: Sandbox + 'static>(mut self, sandbox: S) -> Self {
        self.sandbox = Arc::new(sandbox);
        self
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config.agent.system_prompt = Some(prompt.into());
        self
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.config.agent.model = Some(model.into());
        self
    }

    /// Get the execution history.
    pub fn execution_history(&self) -> &[CodeExecution] {
        &self.execution_history
    }

    /// Execute code directly.
    pub async fn execute_code(
        &mut self,
        code: &str,
        language: Option<Language>,
    ) -> AgentResult<ExecutionResult> {
        let language = language
            .or_else(|| {
                if self.config.auto_detect_language {
                    Language::detect(code)
                } else {
                    None
                }
            })
            .or(self.config.default_language)
            .ok_or_else(|| {
                AgentError::ConfigError("Could not determine language for code".to_string())
            })?;

        if code.len() > self.config.max_code_length {
            return Err(AgentError::ConfigError(format!(
                "Code exceeds maximum length of {} characters",
                self.config.max_code_length
            )));
        }

        let result = self
            .sandbox
            .execute(code, language, &self.config.sandbox)
            .await
            .map_err(|e| AgentError::Other(format!("Sandbox error: {}", e)))?;

        self.execution_history.push(CodeExecution {
            code: code.to_string(),
            language,
            result: result.clone(),
        });

        Ok(result)
    }

    /// Get the tool definition for code execution.
    fn code_execution_tool() -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "code".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The code to execute"
            }),
        );
        properties.insert(
            "language".to_string(),
            serde_json::json!({
                "type": "string",
                "enum": ["python", "javascript", "shell", "ruby"],
                "description": "The programming language (optional, will auto-detect if not provided)"
            }),
        );

        ToolDefinition {
            tool_type: "function".to_string(),
            function: crate::types::FunctionDefinition {
                name: "execute_code".to_string(),
                description: Some(
                    "Execute code in a sandboxed environment. Supports Python, JavaScript, Shell, and Ruby."
                        .to_string(),
                ),
                parameters: Some(ToolParameters {
                    schema_type: "object".to_string(),
                    properties,
                    required: Some(vec!["code".to_string()]),
                }),
            },
        }
    }

    async fn handle_tool_call(&mut self, tool_call: &ToolCall) -> AgentResult<String> {
        if tool_call.function.name != "execute_code" {
            return Err(AgentError::ToolError {
                tool_name: tool_call.function.name.clone(),
                message: "Unknown tool".to_string(),
            });
        }

        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).map_err(|e| {
                AgentError::ToolError {
                    tool_name: tool_call.function.name.clone(),
                    message: format!("Invalid arguments: {}", e),
                }
            })?;

        let code = args["code"].as_str().ok_or_else(|| AgentError::ToolError {
            tool_name: tool_call.function.name.clone(),
            message: "Missing 'code' argument".to_string(),
        })?;

        let language = args["language"]
            .as_str()
            .and_then(|l| match l.to_lowercase().as_str() {
                "python" => Some(Language::Python),
                "javascript" | "js" => Some(Language::JavaScript),
                "shell" | "bash" | "sh" => Some(Language::Shell),
                "ruby" | "rb" => Some(Language::Ruby),
                _ => None,
            });

        let result = self.execute_code(code, language).await?;

        Ok(serde_json::json!({
            "success": result.success(),
            "exit_code": result.exit_code,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "execution_time_ms": result.execution_time_ms,
            "timed_out": result.timed_out
        })
        .to_string())
    }
}

#[async_trait]
impl Agent for CodeAgent {
    fn name(&self) -> &str {
        &self.config.agent.name
    }

    fn config(&self) -> &AgentConfig {
        &self.config.agent
    }

    fn context_mut(&mut self) -> &mut AgentContext {
        &mut self.context
    }

    fn context(&self) -> &AgentContext {
        &self.context
    }

    async fn step(&mut self) -> AgentResult<AgentStep> {
        let start = Instant::now();
        let step_number = self.context.current_step;

        // Build messages
        let mut messages = Vec::new();

        // Add system prompt
        if let Some(ref prompt) = self.config.agent.system_prompt {
            messages.push(Message::system(prompt));
        }

        // Add conversation history
        messages.extend(self.context.memory.messages().iter().cloned());

        // Get model
        let model = self
            .config
            .agent
            .model
            .clone()
            .unwrap_or_else(|| "gpt-4".to_string());

        // Build request
        let mut request = ChatCompletionRequest::new(model, messages);

        // Add code execution tool
        request = request.tools(vec![Self::code_execution_tool()]);

        // Set temperature if configured
        if let Some(temp) = self.config.agent.temperature {
            request = request.temperature(temp);
        }

        // Make the request
        let response = self.client.chat_completions(request).await?;

        let choice = response
            .choices
            .first()
            .ok_or_else(|| AgentError::Other("No response from LLM".to_string()))?;

        // Check for tool calls
        if let Some(ref tool_calls) = choice.message.tool_calls {
            if !tool_calls.is_empty() {
                // Add assistant message with tool calls to memory
                self.context.memory.add_message(choice.message.clone());

                // Execute tool calls
                let mut tool_results = Vec::new();
                for tool_call in tool_calls {
                    let result = self.handle_tool_call(tool_call).await?;
                    tool_results.push((tool_call.id.clone(), result));
                }

                // Add tool results to memory
                for (id, result) in &tool_results {
                    self.context.memory.add_message(Message::tool(id, result));
                }

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

                return Ok(step);
            }
        }

        // No tool calls - this is the final response
        let content = choice.message.content.clone().unwrap_or_default();

        // Add to memory
        self.context.memory.add_message(choice.message.clone());
        self.context.state = AgentState::Completed;

        let step = AgentStep::new(step_number, StepType::Response)
            .with_result(StepResult::Done { response: content })
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

        Ok(step)
    }

    fn reset(&mut self) {
        self.context.reset();
        self.execution_history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::sandbox::MockSandbox;

    #[test]
    fn test_code_agent_config() {
        let config = CodeAgentConfig::new()
            .with_model("gpt-4")
            .with_default_language(Language::Python);

        assert_eq!(config.agent.model, Some("gpt-4".to_string()));
        assert_eq!(config.default_language, Some(Language::Python));
    }

    #[test]
    fn test_code_execution_tool() {
        let tool = CodeAgent::code_execution_tool();
        assert_eq!(tool.function.name, "execute_code");
        assert!(tool.function.description.is_some());
    }

    #[tokio::test]
    async fn test_code_agent_execute_code() {
        let client = AsyncForgeClient::new();
        let mock_sandbox = MockSandbox::new().with_result(ExecutionResult {
            exit_code: 0,
            stdout: "Hello, World!".to_string(),
            stderr: String::new(),
            execution_time_ms: 10,
            timed_out: false,
            output_files: Vec::new(),
        });

        let mut agent = CodeAgent::new(client).with_sandbox(mock_sandbox);

        let result = agent
            .execute_code("print('Hello, World!')", Some(Language::Python))
            .await
            .unwrap();

        assert!(result.success());
        assert_eq!(result.stdout, "Hello, World!");
        assert_eq!(agent.execution_history().len(), 1);
    }

    #[test]
    fn test_code_execution_record() {
        let record = CodeExecution {
            code: "print(42)".to_string(),
            language: Language::Python,
            result: ExecutionResult {
                exit_code: 0,
                stdout: "42\n".to_string(),
                stderr: String::new(),
                execution_time_ms: 5,
                timed_out: false,
                output_files: Vec::new(),
            },
        };

        assert!(record.result.success());
    }
}
