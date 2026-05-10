use napi::bindgen_prelude::*;
use serde_json::Value as JsonValue;
use liteforge::tools::{
    validate_json_schema as rust_validate_json_schema, Tool as RustTool,
    ToolExecutor as RustToolExecutor, ToolRegistry as RustToolRegistry,
    ToolResult as RustToolResult,
};

struct JsCallableTool {
    name: String,
    description: String,
    parameters: JsonValue,
    requires_confirm: bool,
}

impl RustTool for JsCallableTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> JsonValue {
        self.parameters.clone()
    }

    fn execute(&self, args: JsonValue) -> std::result::Result<JsonValue, String> {
        Ok(args)
    }

    fn requires_confirmation(&self) -> bool {
        self.requires_confirm
    }
}

#[napi(object)]
pub struct JsToolResultData {
    pub tool_call_id: Option<String>,
    pub name: String,
    pub success: bool,
    pub result: Option<String>,
    pub error: Option<String>,
    pub execution_time_ms: Option<f64>,
}

fn rust_tool_result_to_js(r: &RustToolResult) -> JsToolResultData {
    JsToolResultData {
        tool_call_id: Some(r.tool_call_id.clone()),
        name: r.name.clone(),
        success: r.success,
        result: r
            .result
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default()),
        error: r.error.clone(),
        execution_time_ms: r.execution_time_ms.map(|ms| ms as f64),
    }
}

#[napi(object)]
pub struct SchemaValidationError {
    pub path: String,
    pub message: String,
}

#[napi]
pub fn validate_json_schema(
    value: serde_json::Value,
    schema: serde_json::Value,
) -> Vec<SchemaValidationError> {
    match rust_validate_json_schema(&value, &schema) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .into_iter()
            .map(|e| SchemaValidationError {
                path: e.path.clone(),
                message: e.message.clone(),
            })
            .collect(),
    }
}

#[napi]
pub struct ToolRegistry {
    inner: RustToolRegistry,
}

#[napi]
impl ToolRegistry {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustToolRegistry::new(),
        }
    }

    #[napi]
    pub fn register_tool(
        &mut self,
        name: String,
        description: String,
        parameters: serde_json::Value,
    ) {
        let tool = JsCallableTool {
            name,
            description,
            parameters,
            requires_confirm: false,
        };
        self.inner.register(Box::new(tool));
    }

    #[napi]
    pub fn unregister(&mut self, name: String) -> bool {
        self.inner.unregister(&name).is_some()
    }

    #[napi]
    pub fn contains(&self, name: String) -> bool {
        self.inner.contains(&name)
    }

    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    #[napi]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[napi]
    pub fn names(&self) -> Vec<String> {
        self.inner
            .names()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[napi]
    pub fn definitions(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.definitions()).unwrap_or(serde_json::Value::Array(vec![]))
    }
}

#[napi]
pub struct ToolExecutor {
    inner: RustToolExecutor,
}

#[napi]
impl ToolExecutor {
    #[napi(constructor)]
    pub fn new(registry: &ToolRegistry) -> Self {
        Self {
            inner: RustToolExecutor::new(registry.inner.clone()),
        }
    }

    #[napi]
    pub fn has_tool(&self, name: String) -> bool {
        self.inner.has_tool(&name)
    }

    #[napi]
    pub fn execute(&self, name: String, args: serde_json::Value) -> Result<JsToolResultData> {
        let result = self.inner.execute(&name, args);
        Ok(rust_tool_result_to_js(&result))
    }

    #[napi]
    pub fn execute_with_id(
        &self,
        tool_call_id: String,
        name: String,
        args: serde_json::Value,
    ) -> Result<JsToolResultData> {
        let result = self.inner.execute_with_id(&tool_call_id, &name, args);
        Ok(rust_tool_result_to_js(&result))
    }
}
