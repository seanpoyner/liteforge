use liteforge::{
    ChatCompletion as RustChatCompletion, ChatCompletionChunk as RustChatCompletionChunk,
    Choice as RustChoice, EmbeddingResponse as RustEmbeddingResponse,
    FunctionCall as RustFunctionCall, FunctionDefinition as RustFunctionDefinition,
    Message as RustMessage, Model as RustModel, ModelList as RustModelList,
    ToolCall as RustToolCall, ToolDefinition as RustToolDefinition,
    ToolParameters as RustToolParameters, Usage as RustUsage,
};

#[napi(object)]
pub struct Message {
    pub role: String,
    pub content: Option<String>,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<JsToolCall>>,
    pub tool_call_id: Option<String>,
}

#[napi(object)]
pub struct JsToolCall {
    pub index: Option<u32>,
    pub id: String,
    pub call_type: String,
    pub function: JsFunctionCall,
}

#[napi(object)]
pub struct JsFunctionCall {
    pub name: String,
    pub arguments: String,
}

#[napi(object)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[napi(object)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[napi(object)]
pub struct ChatCompletion {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[napi(object)]
pub struct ChoiceDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<JsToolCall>>,
}

#[napi(object)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: ChoiceDelta,
    pub finish_reason: Option<String>,
}

#[napi(object)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

#[napi(object)]
pub struct JsModel {
    pub id: String,
    pub object: String,
    pub created: Option<i64>,
    pub owned_by: Option<String>,
}

#[napi(object)]
pub struct JsModelList {
    pub object: String,
    pub data: Vec<JsModel>,
}

#[napi(object)]
pub struct JsEmbeddingData {
    pub object: String,
    pub embedding: Vec<f64>,
    pub index: u32,
}

#[napi(object)]
pub struct JsEmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

#[napi(object)]
pub struct JsEmbeddingResponse {
    pub object: String,
    pub data: Vec<JsEmbeddingData>,
    pub model: String,
    pub usage: JsEmbeddingUsage,
}

#[napi(object)]
pub struct JsToolDefinition {
    pub tool_type: String,
    pub function: JsFunctionDefinition,
}

#[napi(object)]
pub struct JsFunctionDefinition {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

pub fn rust_message_to_js(msg: &RustMessage) -> Message {
    Message {
        role: msg.role.clone(),
        content: msg.content.clone(),
        name: msg.name.clone(),
        tool_calls: msg
            .tool_calls
            .as_ref()
            .map(|calls| calls.iter().map(rust_tool_call_to_js).collect()),
        tool_call_id: msg.tool_call_id.clone(),
    }
}

pub fn js_message_to_rust(msg: &Message) -> RustMessage {
    RustMessage {
        role: msg.role.clone(),
        content: msg.content.clone(),
        name: msg.name.clone(),
        tool_calls: msg
            .tool_calls
            .as_ref()
            .map(|calls| calls.iter().map(js_tool_call_to_rust).collect()),
        tool_call_id: msg.tool_call_id.clone(),
    }
}

fn rust_tool_call_to_js(tc: &RustToolCall) -> JsToolCall {
    JsToolCall {
        index: tc.index,
        id: tc.id.clone(),
        call_type: tc.call_type.clone(),
        function: JsFunctionCall {
            name: tc.function.name.clone(),
            arguments: tc.function.arguments.clone(),
        },
    }
}

fn js_tool_call_to_rust(tc: &JsToolCall) -> RustToolCall {
    RustToolCall {
        index: tc.index,
        id: tc.id.clone(),
        call_type: tc.call_type.clone(),
        function: RustFunctionCall {
            name: tc.function.name.clone(),
            arguments: tc.function.arguments.clone(),
        },
    }
}

pub fn rust_choice_to_js(c: &RustChoice) -> Choice {
    Choice {
        index: c.index,
        message: rust_message_to_js(&c.message),
        finish_reason: c.finish_reason.clone(),
    }
}

pub fn rust_usage_to_js(u: &RustUsage) -> Usage {
    Usage {
        prompt_tokens: u.prompt_tokens,
        completion_tokens: u.completion_tokens,
        total_tokens: u.total_tokens,
    }
}

pub fn rust_completion_to_js(c: &RustChatCompletion) -> ChatCompletion {
    ChatCompletion {
        id: c.id.clone(),
        object: c.object.clone(),
        created: c.created,
        model: c.model.clone(),
        choices: c.choices.iter().map(rust_choice_to_js).collect(),
        usage: c.usage.as_ref().map(rust_usage_to_js),
    }
}

pub fn rust_chunk_to_js(c: &RustChatCompletionChunk) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: c.id.clone(),
        object: c.object.clone(),
        created: c.created,
        model: c.model.clone(),
        choices: c
            .choices
            .iter()
            .map(|sc| StreamChoice {
                index: sc.index,
                delta: ChoiceDelta {
                    role: sc.delta.role.clone(),
                    content: sc.delta.content.clone(),
                    tool_calls: sc
                        .delta
                        .tool_calls
                        .as_ref()
                        .map(|calls| calls.iter().map(rust_tool_call_to_js).collect()),
                },
                finish_reason: sc.finish_reason.clone(),
            })
            .collect(),
    }
}

pub fn rust_model_to_js(m: &RustModel) -> JsModel {
    JsModel {
        id: m.id.clone(),
        object: m.object.clone(),
        created: m.created,
        owned_by: m.owned_by.clone(),
    }
}

pub fn rust_model_list_to_js(ml: &RustModelList) -> JsModelList {
    JsModelList {
        object: ml.object.clone(),
        data: ml.data.iter().map(rust_model_to_js).collect(),
    }
}

pub fn rust_embedding_response_to_js(r: &RustEmbeddingResponse) -> JsEmbeddingResponse {
    JsEmbeddingResponse {
        object: r.object.clone(),
        data: r
            .data
            .iter()
            .map(|d| JsEmbeddingData {
                object: d.object.clone(),
                embedding: d.embedding.iter().map(|&v| v as f64).collect(),
                index: d.index as u32,
            })
            .collect(),
        model: r.model.clone(),
        usage: JsEmbeddingUsage {
            prompt_tokens: r.usage.prompt_tokens,
            total_tokens: r.usage.total_tokens,
        },
    }
}

pub fn js_tool_def_to_rust(td: &JsToolDefinition) -> RustToolDefinition {
    RustToolDefinition {
        tool_type: td.tool_type.clone(),
        function: RustFunctionDefinition {
            name: td.function.name.clone(),
            description: td.function.description.clone(),
            parameters: td
                .function
                .parameters
                .as_ref()
                .and_then(|p| serde_json::from_value::<RustToolParameters>(p.clone()).ok()),
        },
    }
}

#[napi]
pub fn create_message_user(content: String) -> Message {
    rust_message_to_js(&RustMessage::user(content))
}

#[napi]
pub fn create_message_system(content: String) -> Message {
    rust_message_to_js(&RustMessage::system(content))
}

#[napi]
pub fn create_message_assistant(content: String) -> Message {
    rust_message_to_js(&RustMessage::assistant(content))
}

#[napi]
pub fn create_message_tool(tool_call_id: String, content: String) -> Message {
    rust_message_to_js(&RustMessage::tool(tool_call_id, content))
}
