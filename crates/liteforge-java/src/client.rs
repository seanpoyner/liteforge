//! JNI bindings for ForgeClient.

use crate::error::{handle_result, throw_exception, JavaBindingError, Result};
use crate::types::{completion_to_jobject, jlist_to_messages, jstring_to_string};
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jint, jlong};
use jni::JNIEnv;
use std::sync::Arc;
use liteforge::{AsyncForgeClient, Message, ForgeClient, ForgeConfig};
use tokio::runtime::Runtime;

pub(crate) struct ClientHandle {
    pub(crate) client: ForgeClient,
    pub(crate) async_client: Arc<AsyncForgeClient>,
    pub(crate) runtime: Arc<Runtime>,
}

impl ClientHandle {
    fn new() -> Result<Self> {
        let runtime = Runtime::new().map_err(|e| {
            JavaBindingError::InvalidArgument(format!("Failed to create tokio runtime: {}", e))
        })?;
        let client = ForgeClient::new();
        let async_client = AsyncForgeClient::new();
        Ok(Self {
            client,
            async_client: Arc::new(async_client),
            runtime: Arc::new(runtime),
        })
    }

    fn with_config(config: ForgeConfig) -> Result<Self> {
        let runtime = Runtime::new().map_err(|e| {
            JavaBindingError::InvalidArgument(format!("Failed to create tokio runtime: {}", e))
        })?;
        let client = ForgeClient::with_config(config.clone());
        let async_client = AsyncForgeClient::with_config(config);
        Ok(Self {
            client,
            async_client: Arc::new(async_client),
            runtime: Arc::new(runtime),
        })
    }
}

pub(crate) fn get_handle(ptr: jlong) -> Result<&'static ClientHandle> {
    if ptr == 0 {
        return Err(JavaBindingError::NullPointer(
            "Client handle is null".to_string(),
        ));
    }
    unsafe { Ok(&*(ptr as *const ClientHandle)) }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ForgeClient_nativeCreate(
    mut env: JNIEnv,
    _class: JClass,
) -> jlong {
    let result = ClientHandle::new().map(|h| Box::into_raw(Box::new(h)) as jlong);
    handle_result(&mut env, result, 0)
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ForgeClient_nativeCreateWithConfig(
    mut env: JNIEnv,
    _class: JClass,
    api_key: JString,
    base_url: JString,
    default_model: JString,
    timeout_secs: jint,
) -> jlong {
    let result = (|| -> Result<jlong> {
        let mut builder = ForgeConfig::builder();

        if !api_key.is_null() {
            let key = jstring_to_string(&mut env, &api_key)?;
            builder = builder.api_key(key);
        }

        if !base_url.is_null() {
            let url = jstring_to_string(&mut env, &base_url)?;
            builder = builder.base_url(url);
        }

        if !default_model.is_null() {
            let model = jstring_to_string(&mut env, &default_model)?;
            builder = builder.default_model(model);
        }

        if timeout_secs > 0 {
            builder = builder.timeout_secs(timeout_secs as u64);
        }

        let config = builder.build();
        let handle = ClientHandle::with_config(config)?;
        Ok(Box::into_raw(Box::new(handle)) as jlong)
    })();

    handle_result(&mut env, result, 0)
}

/// Create a client from a fully-specified JSON config blob.
///
/// JNI-side equivalent of the Python/JS expanded constructors. The Java
/// `ForgeConfig.Builder` serialises its state to JSON and calls this, much
/// simpler than walking Java `Map<String, Object>` via JNI.
///
/// Expected JSON shape (all fields optional; missing fields fall back to
/// env vars or defaults):
/// ```json
/// {
///   "api_key": "...",
///   "base_url": "...",
///   "default_model": "...",
///   "timeout_secs": 60,
///   "default_headers": {"X-App-Id": "btsales"},
///   "default_metadata": {"app": "btsales", "env": "preprod"},
///   "otel": {
///     "endpoint": "...",
///     "headers": {"Authorization": "Api-Token <token>"},
///     "service_name": "btsales-agent",
///     "resource_attributes": {"deployment.environment": "preprod"},
///     "capture_prompts": false
///   }
/// }
/// ```
#[no_mangle]
pub extern "system" fn Java_com_liteforge_ForgeClient_nativeCreateWithJsonConfig(
    mut env: JNIEnv,
    _class: JClass,
    json_config: JString,
) -> jlong {
    let result = (|| -> Result<jlong> {
        if json_config.is_null() {
            return Err(JavaBindingError::InvalidArgument(
                "json_config must not be null".to_string(),
            ));
        }
        let json_str = jstring_to_string(&mut env, &json_config)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
            JavaBindingError::InvalidArgument(format!("invalid JSON config: {}", e))
        })?;

        let mut builder = ForgeConfig::builder();
        if let Some(s) = parsed.get("api_key").and_then(|v| v.as_str()) {
            builder = builder.api_key(s.to_string());
        }
        if let Some(s) = parsed.get("base_url").and_then(|v| v.as_str()) {
            builder = builder.base_url(s.to_string());
        }
        if let Some(s) = parsed.get("default_model").and_then(|v| v.as_str()) {
            builder = builder.default_model(s.to_string());
        }
        if let Some(t) = parsed.get("timeout_secs").and_then(|v| v.as_u64()) {
            builder = builder.timeout_secs(t);
        }
        if let Some(headers) = parsed.get("default_headers").and_then(|v| v.as_object()) {
            let map: std::collections::HashMap<String, String> = headers
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect();
            if !map.is_empty() {
                builder = builder.default_headers(map);
            }
        }
        if let Some(meta) = parsed.get("default_metadata").and_then(|v| v.as_object()) {
            let map: std::collections::HashMap<String, serde_json::Value> =
                meta.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            if !map.is_empty() {
                builder = builder.default_metadata(map);
            }
        }
        if let Some(otel_obj) = parsed.get("otel").and_then(|v| v.as_object()) {
            let endpoint = otel_obj
                .get("endpoint")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let headers = otel_obj
                .get("headers")
                .and_then(|v| v.as_object())
                .map(|h| {
                    h.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            let service_name = otel_obj
                .get("service_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let resource_attributes = otel_obj
                .get("resource_attributes")
                .and_then(|v| v.as_object())
                .map(|r| {
                    r.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            let capture_prompts = otel_obj
                .get("capture_prompts")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            builder = builder.otel(liteforge::OtelConfig {
                endpoint,
                headers,
                service_name,
                resource_attributes,
                capture_prompts,
            });
        }

        let config = builder.build();
        let handle = ClientHandle::with_config(config)?;
        Ok(Box::into_raw(Box::new(handle)) as jlong)
    })();

    handle_result(&mut env, result, 0)
}

/// Initialise the global OTel tracer provider + W3C propagator from Java.
/// Accepts a JSON-encoded `OtelConfig` blob (same shape as the `otel`
/// field above). Idempotent; no-op when built without `--features otel`.
#[no_mangle]
pub extern "system" fn Java_com_liteforge_ForgeClient_nativeInitOtel(
    mut env: JNIEnv,
    _class: JClass,
    json_otel_config: JString,
) -> jint {
    let result = (|| -> Result<()> {
        if json_otel_config.is_null() {
            liteforge::init_otel(&liteforge::OtelConfig::default()).map_err(|e| {
                JavaBindingError::InvalidArgument(format!("init_otel failed: {}", e))
            })?;
            return Ok(());
        }
        let json_str = jstring_to_string(&mut env, &json_otel_config)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
            JavaBindingError::InvalidArgument(format!("invalid JSON otel config: {}", e))
        })?;
        let obj = parsed
            .as_object()
            .ok_or_else(|| JavaBindingError::InvalidArgument("expected JSON object".into()))?;

        let endpoint = obj
            .get("endpoint")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let headers = obj
            .get("headers")
            .and_then(|v| v.as_object())
            .map(|h| {
                h.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        let service_name = obj
            .get("service_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let resource_attributes = obj
            .get("resource_attributes")
            .and_then(|v| v.as_object())
            .map(|r| {
                r.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        let capture_prompts = obj
            .get("capture_prompts")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let otel = liteforge::OtelConfig {
            endpoint,
            headers,
            service_name,
            resource_attributes,
            capture_prompts,
        };

        liteforge::init_otel(&otel)
            .map_err(|e| JavaBindingError::InvalidArgument(format!("init_otel failed: {}", e)))?;
        Ok(())
    })();

    handle_result(&mut env, result.map(|_| 0i32), -1)
}

/// Returns 1 if the native library was built with `--features otel`,
/// 0 otherwise. When 0, [`nativeInitOtel`] is a no-op.
#[no_mangle]
pub extern "system" fn Java_com_liteforge_ForgeClient_nativeOtelFeatureEnabled(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    if liteforge::otel_feature_enabled() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ForgeClient_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut ClientHandle);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ForgeClient_nativeGetModel<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
) -> JString<'local> {
    let result = (|| -> Result<JString<'local>> {
        let h = get_handle(handle)?;
        let model = h.client.model();
        let jstr = env.new_string(model)?;
        Ok(jstr)
    })();

    match result {
        Ok(s) => s,
        Err(e) => {
            throw_exception(&mut env, e);
            JString::default()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ForgeClient_nativeGetBaseUrl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
) -> JString<'local> {
    let result = (|| -> Result<JString<'local>> {
        let h = get_handle(handle)?;
        let url = h.client.base_url();
        let jstr = env.new_string(url)?;
        Ok(jstr)
    })();

    match result {
        Ok(s) => s,
        Err(e) => {
            throw_exception(&mut env, e);
            JString::default()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ForgeClient_nativeComplete<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
    messages: JObject,
) -> JObject<'local> {
    let result = (|| -> Result<JObject<'local>> {
        let h = get_handle(handle)?;
        let msgs = jlist_to_messages(&mut env, &messages)?;
        let completion = h.client.complete(msgs)?;
        completion_to_jobject(&mut env, &completion)
    })();

    match result {
        Ok(obj) => obj,
        Err(e) => {
            throw_exception(&mut env, e);
            JObject::null()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ForgeClient_nativeCompleteWithModel<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
    model: JString,
    messages: JObject,
) -> JObject<'local> {
    let result = (|| -> Result<JObject<'local>> {
        let h = get_handle(handle)?;
        let model_str = jstring_to_string(&mut env, &model)?;
        let msgs = jlist_to_messages(&mut env, &messages)?;
        let completion = h.client.complete_with_model(&model_str, msgs)?;
        completion_to_jobject(&mut env, &completion)
    })();

    match result {
        Ok(obj) => obj,
        Err(e) => {
            throw_exception(&mut env, e);
            JObject::null()
        }
    }
}

fn spawn_async_completion(
    env: &mut JNIEnv,
    handle: jlong,
    msgs: Vec<Message>,
    callback: JObject,
    model: Option<String>,
) -> Result<()> {
    let h = get_handle(handle)?;

    let callback_global = env.new_global_ref(callback)?;
    let jvm = env.get_java_vm()?;

    let async_client = Arc::clone(&h.async_client);
    let runtime = Arc::clone(&h.runtime);

    runtime.spawn(async move {
        let result = match model {
            Some(m) => async_client.complete_with_model(&m, msgs).await,
            None => async_client.complete(msgs).await,
        };

        let mut env = match jvm.attach_current_thread() {
            Ok(env) => env,
            Err(_) => return,
        };

        match result {
            Ok(completion) => {
                if let Ok(completion_obj) = completion_to_jobject(&mut env, &completion) {
                    let _ = env.call_method(
                        &callback_global,
                        "onSuccess",
                        "(Lcom/liteforge/ChatCompletion;)V",
                        &[(&completion_obj).into()],
                    );
                }
            }
            Err(e) => {
                if let Ok(error_msg) = env.new_string(e.to_string()) {
                    let _ = env.call_method(
                        &callback_global,
                        "onError",
                        "(Ljava/lang/String;)V",
                        &[(&error_msg).into()],
                    );
                }
            }
        }
    });

    Ok(())
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ForgeClient_nativeCompleteAsync(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    messages: JObject,
    callback: JObject,
) {
    let result = (|| -> Result<()> {
        let msgs = jlist_to_messages(&mut env, &messages)?;
        spawn_async_completion(&mut env, handle, msgs, callback, None)
    })();

    if let Err(e) = result {
        throw_exception(&mut env, e);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ForgeClient_nativeCompleteAsyncWithModel(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    model: JString,
    messages: JObject,
    callback: JObject,
) {
    let result = (|| -> Result<()> {
        let model_str = jstring_to_string(&mut env, &model)?;
        let msgs = jlist_to_messages(&mut env, &messages)?;
        spawn_async_completion(&mut env, handle, msgs, callback, Some(model_str))
    })();

    if let Err(e) = result {
        throw_exception(&mut env, e);
    }
}
