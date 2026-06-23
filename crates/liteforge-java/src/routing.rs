//! JNI bindings for native model routing (config-driven).
//!
//! Handle-based like the client: a boxed [`RouterHandle`] pointer is returned to
//! Java as a `jlong`. Returns are plain strings (JSON for the route decision) to
//! keep the JNI surface small and Java dependency-free.

use crate::error::{throw_exception, JavaBindingError, Result};
use crate::types::jstring_to_string;
use jni::objects::{JClass, JString};
use jni::sys::jlong;
use jni::JNIEnv;
use liteforge::model_routing::ModelRoutingConfig;
use liteforge::routing::Router as CoreRouter;
use liteforge::{ChatCompletionRequest, Message};
use std::sync::Arc;
use tokio::runtime::Runtime;

pub(crate) struct RouterHandle {
    router: Arc<CoreRouter>,
    runtime: Arc<Runtime>,
}

fn get_router(handle: jlong) -> Result<&'static RouterHandle> {
    if handle == 0 {
        return Err(JavaBindingError::InvalidArgument(
            "null router handle".to_string(),
        ));
    }
    unsafe { Ok(&*(handle as *const RouterHandle)) }
}

fn build_handle(yaml: &str) -> Result<RouterHandle> {
    let runtime = Runtime::new()
        .map_err(|e| JavaBindingError::InvalidArgument(format!("tokio runtime: {e}")))?;
    let mut router = CoreRouter::from_yaml_str(yaml)?;
    if let Some(mr) = ModelRoutingConfig::parse_optional(yaml)? {
        let selector = runtime.block_on(mr.build_selector())?;
        router = router.with_selector(Arc::from(selector));
    }
    Ok(RouterHandle {
        router: Arc::new(router),
        runtime: Arc::new(runtime),
    })
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_Router_nativeCreateFromYaml(
    mut env: JNIEnv,
    _class: JClass,
    yaml: JString,
) -> jlong {
    let result = (|| -> Result<jlong> {
        let yaml = jstring_to_string(&mut env, &yaml)?;
        let handle = build_handle(&yaml)?;
        Ok(Box::into_raw(Box::new(handle)) as jlong)
    })();
    match result {
        Ok(h) => h,
        Err(e) => {
            throw_exception(&mut env, e);
            0
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_Router_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut RouterHandle);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_Router_nativeWhichModel<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
    prompt: JString,
) -> JString<'local> {
    let result = (|| -> Result<JString<'local>> {
        let h = get_router(handle)?;
        let prompt = jstring_to_string(&mut env, &prompt)?;
        let model = h.runtime.block_on(h.router.which_model(prompt))?;
        Ok(env.new_string(model)?)
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
pub extern "system" fn Java_com_liteforge_Router_nativeRouteJson<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
    prompt: JString,
) -> JString<'local> {
    let result = (|| -> Result<JString<'local>> {
        let h = get_router(handle)?;
        let prompt = jstring_to_string(&mut env, &prompt)?;
        let req = ChatCompletionRequest::new("auto", vec![Message::user(prompt)]);
        let d = h.runtime.block_on(h.router.route_decision(&req))?;
        let json = serde_json::json!({
            "group": d.group,
            "model": d.model,
            "base_url": d.base_url,
            "strategy": d.strategy,
            "score": d.score,
            "fallback_chain": d.fallback_chain,
        })
        .to_string();
        Ok(env.new_string(json)?)
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
pub extern "system" fn Java_com_liteforge_Router_nativeStrategy<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
) -> JString<'local> {
    let result = (|| -> Result<JString<'local>> {
        let h = get_router(handle)?;
        Ok(env.new_string(h.router.strategy_name())?)
    })();
    match result {
        Ok(s) => s,
        Err(e) => {
            throw_exception(&mut env, e);
            JString::default()
        }
    }
}

/// Returns the model groups as a comma-separated string (split in Java).
#[no_mangle]
pub extern "system" fn Java_com_liteforge_Router_nativeModelGroups<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
) -> JString<'local> {
    let result = (|| -> Result<JString<'local>> {
        let h = get_router(handle)?;
        let groups = h.router.model_groups().join(",");
        Ok(env.new_string(groups)?)
    })();
    match result {
        Ok(s) => s,
        Err(e) => {
            throw_exception(&mut env, e);
            JString::default()
        }
    }
}
