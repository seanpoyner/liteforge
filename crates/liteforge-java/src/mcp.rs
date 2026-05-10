//! JNI bindings for MCP (Model Context Protocol) servers.
//!
//! Design:
//! - Java passes a `McpServerConfig` as a JSON string. The Rust side
//!   deserializes it, inspects the transport, and constructs the appropriate
//!   `McpStdioServer`/`McpHttpServer`/`McpSseServer`, wrapping it in an
//!   `Arc<tokio::sync::Mutex<Box<dyn McpServer>>>`.
//! - All async operations block on the shared `ForgeClient` runtime.
//! - `registerTools` bridges MCP tools into a Java `ToolRegistry` by adding
//!   Rust-side `McpBridgedTool`s that call back into the server. When the
//!   agent loop executes one of these tools, it uses `block_in_place` so the
//!   tokio worker thread doesn't deadlock while awaiting the MCP call.

use crate::client::get_handle;
use crate::error::{throw_exception, JavaBindingError, Result};
use crate::tools::registry_from_handle;
use crate::types::jstring_to_string;
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jlong, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use liteforge::mcp::{
    CallToolParams, McpHttpServer, McpServer as RustMcpServer, McpServerConfig, McpSseServer,
    McpStdioServer, ToolResultContent, TransportType,
};
use liteforge::tools::Tool as RustTool;
use tokio::sync::Mutex as AsyncMutex;

type SharedServer = Arc<AsyncMutex<Box<dyn RustMcpServer>>>;

pub(crate) struct McpServerHandle {
    server: SharedServer,
    runtime: Arc<tokio::runtime::Runtime>,
}

fn server_from_handle(ptr: jlong) -> Result<&'static McpServerHandle> {
    if ptr == 0 {
        return Err(JavaBindingError::NullPointer(
            "McpServer handle is null".into(),
        ));
    }
    Ok(unsafe { &*(ptr as *const McpServerHandle) })
}

fn build_server(config: McpServerConfig) -> Box<dyn RustMcpServer> {
    match config.transport {
        TransportType::Stdio => Box::new(McpStdioServer::new(config)),
        TransportType::Http => Box::new(McpHttpServer::new(config)),
        TransportType::Sse => Box::new(McpSseServer::new(config)),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_McpServer_nativeCreate(
    mut env: JNIEnv,
    _class: JClass,
    client_handle: jlong,
    config_json: JString,
) -> jlong {
    let res = (|| -> Result<jlong> {
        let client = get_handle(client_handle)?;
        let cfg_str = jstring_to_string(&mut env, &config_json)?;
        let config: McpServerConfig = serde_json::from_str(&cfg_str).map_err(|e| {
            JavaBindingError::InvalidArgument(format!("Invalid McpServerConfig JSON: {e}"))
        })?;
        let server = build_server(config);
        let handle = McpServerHandle {
            server: Arc::new(AsyncMutex::new(server)),
            runtime: Arc::clone(&client.runtime),
        };
        Ok(Box::into_raw(Box::new(handle)) as jlong)
    })();
    match res {
        Ok(h) => h,
        Err(e) => {
            throw_exception(&mut env, e);
            0
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_McpServer_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut McpServerHandle);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_McpServer_nativeConnect(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    let res = (|| -> Result<()> {
        let h = server_from_handle(handle)?;
        let server = Arc::clone(&h.server);
        h.runtime
            .block_on(async move {
                let mut guard = server.lock().await;
                guard.connect().await
            })
            .map_err(|e| JavaBindingError::InvalidArgument(format!("MCP connect: {e}")))?;
        Ok(())
    })();
    if let Err(e) = res {
        throw_exception(&mut env, e);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_McpServer_nativeDisconnect(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    let res = (|| -> Result<()> {
        let h = server_from_handle(handle)?;
        let server = Arc::clone(&h.server);
        h.runtime
            .block_on(async move {
                let mut guard = server.lock().await;
                guard.disconnect().await
            })
            .map_err(|e| JavaBindingError::InvalidArgument(format!("MCP disconnect: {e}")))?;
        Ok(())
    })();
    if let Err(e) = res {
        throw_exception(&mut env, e);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_McpServer_nativeIsConnected(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jboolean {
    let res = (|| -> Result<bool> {
        let h = server_from_handle(handle)?;
        let server = Arc::clone(&h.server);
        let connected = h
            .runtime
            .block_on(async move { server.lock().await.is_connected() });
        Ok(connected)
    })();
    match res {
        Ok(true) => JNI_TRUE,
        Ok(false) => JNI_FALSE,
        Err(e) => {
            throw_exception(&mut env, e);
            JNI_FALSE
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_McpServer_nativeListToolsJson<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
) -> JString<'local> {
    let res = (|| -> Result<JString<'local>> {
        let h = server_from_handle(handle)?;
        let server = Arc::clone(&h.server);
        let tools = h
            .runtime
            .block_on(async move {
                let guard = server.lock().await;
                guard.list_tools().await
            })
            .map_err(|e| JavaBindingError::InvalidArgument(format!("MCP list_tools: {e}")))?;

        let json = serde_json::to_string(&tools.tools)
            .map_err(|e| JavaBindingError::InvalidArgument(format!("serialize tools: {e}")))?;
        Ok(env.new_string(&json)?)
    })();
    match res {
        Ok(s) => s,
        Err(e) => {
            throw_exception(&mut env, e);
            JString::default()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_McpServer_nativeCallToolJson<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
    tool_name: JString,
    args_json: JString,
) -> JString<'local> {
    let res = (|| -> Result<JString<'local>> {
        let h = server_from_handle(handle)?;
        let tool = jstring_to_string(&mut env, &tool_name)?;
        let args_str = jstring_to_string(&mut env, &args_json)?;
        let args: JsonValue = serde_json::from_str(&args_str)
            .map_err(|e| JavaBindingError::InvalidArgument(format!("Invalid args JSON: {e}")))?;

        let arguments = args
            .as_object()
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        let params = CallToolParams {
            name: tool,
            arguments,
        };

        let server = Arc::clone(&h.server);
        let result = h
            .runtime
            .block_on(async move {
                let guard = server.lock().await;
                guard.call_tool(params).await
            })
            .map_err(|e| JavaBindingError::InvalidArgument(format!("MCP call_tool: {e}")))?;

        let json = serde_json::to_string(&result)
            .map_err(|e| JavaBindingError::InvalidArgument(format!("serialize result: {e}")))?;
        Ok(env.new_string(&json)?)
    })();
    match res {
        Ok(s) => s,
        Err(e) => {
            throw_exception(&mut env, e);
            JString::default()
        }
    }
}

/// Bridge an MCP tool into the SDK's Tool trait so an agent can call it.
struct McpBridgedTool {
    name: String,
    description: String,
    parameters: JsonValue,
    server: SharedServer,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl RustTool for McpBridgedTool {
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
        let name = self.name.clone();
        let server = Arc::clone(&self.server);
        let runtime = Arc::clone(&self.runtime);

        let arguments = args
            .as_object()
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        let params = CallToolParams { name, arguments };

        // Use a dedicated thread so we can safely block_on even when called
        // from a tokio worker (agent execution path).
        let join = std::thread::spawn(move || {
            runtime.block_on(async move {
                let guard = server.lock().await;
                guard.call_tool(params).await
            })
        })
        .join();

        let result = match join {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => return Err(format!("MCP call_tool failed: {e}")),
            Err(_) => return Err("MCP tool bridge thread panicked".to_string()),
        };

        Ok(tool_result_to_json(&result.content))
    }
}

fn tool_result_to_json(content: &[ToolResultContent]) -> JsonValue {
    if content.len() == 1 {
        match &content[0] {
            ToolResultContent::Text { text } => JsonValue::String(text.clone()),
            ToolResultContent::Image { data, mime_type } => serde_json::json!({
                "type": "image",
                "data": data,
                "mime_type": mime_type
            }),
            ToolResultContent::Resource { resource, text } => serde_json::json!({
                "type": "resource",
                "uri": resource.uri,
                "text": text
            }),
        }
    } else {
        JsonValue::Array(
            content
                .iter()
                .map(|c| match c {
                    ToolResultContent::Text { text } => {
                        serde_json::json!({"type": "text", "text": text})
                    }
                    ToolResultContent::Image { data, mime_type } => {
                        serde_json::json!({"type": "image", "data": data, "mime_type": mime_type})
                    }
                    ToolResultContent::Resource { resource, text } => {
                        serde_json::json!({"type": "resource", "uri": resource.uri, "text": text})
                    }
                })
                .collect(),
        )
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_McpServer_nativeRegisterTools(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    registry_handle: jlong,
) -> jint {
    let res = (|| -> Result<jint> {
        let h = server_from_handle(handle)?;
        let reg = registry_from_handle(registry_handle)?;

        let server = Arc::clone(&h.server);
        let tools = h
            .runtime
            .block_on(async move {
                let guard = server.lock().await;
                guard.list_tools().await
            })
            .map_err(|e| JavaBindingError::InvalidArgument(format!("MCP list_tools: {e}")))?;

        let mut count: jint = 0;
        let mut registry_guard = reg
            .lock()
            .map_err(|e| JavaBindingError::InvalidArgument(format!("registry poisoned: {e}")))?;

        for tool in &tools.tools {
            let parameters = tool.input_schema.clone();
            let bridged = McpBridgedTool {
                name: tool.name.clone(),
                description: tool
                    .description
                    .clone()
                    .unwrap_or_else(|| format!("MCP tool: {}", tool.name)),
                parameters,
                server: Arc::clone(&h.server),
                runtime: Arc::clone(&h.runtime),
            };
            registry_guard.register(Box::new(bridged));
            count += 1;
        }
        Ok(count)
    })();
    match res {
        Ok(n) => n,
        Err(e) => {
            throw_exception(&mut env, e);
            0
        }
    }
}
