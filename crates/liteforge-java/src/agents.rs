//! JNI bindings for ToolCallingAgent.
//!
//! Design:
//! - `AgentConfig` on the Java side is a POJO; it is serialized to JSON and
//!   deserialized into `liteforge::agents::AgentConfig` on the Rust side.
//! - The agent is built from the `ForgeClient`'s shared `AsyncForgeClient` and a
//!   snapshot of the `ToolRegistry`. Tools registered to the registry *after*
//!   the agent is created are NOT seen by that agent — mirrors Python/JS.
//! - `run()` blocks the calling Java thread on the shared tokio runtime;
//!   `runAsync()` spawns on the runtime and dispatches to a Java callback.

#![allow(clippy::redundant_closure)]

use crate::client::get_handle;
use crate::error::{throw_exception, JavaBindingError, Result};
use crate::tools::registry_from_handle;
use crate::types::jstring_to_string;
use jni::objects::{JClass, JObject, JString, JValueGen};
use jni::sys::jlong;
use jni::JNIEnv;
use std::sync::Arc;
use liteforge::agents::{Agent as _, AgentConfig, ToolCallingAgent};
use tokio::sync::Mutex as AsyncMutex;

pub(crate) struct AgentHandle {
    agent: Arc<AsyncMutex<ToolCallingAgent>>,
}

fn handle_from_ptr(ptr: jlong) -> Result<&'static AgentHandle> {
    if ptr == 0 {
        return Err(JavaBindingError::NullPointer("Agent handle is null".into()));
    }
    Ok(unsafe { &*(ptr as *const AgentHandle) })
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolCallingAgent_nativeCreate(
    mut env: JNIEnv,
    _class: JClass,
    client_handle: jlong,
    registry_handle: jlong,
    config_json: JString,
) -> jlong {
    let res = (|| -> Result<jlong> {
        let client = get_handle(client_handle)?;
        let reg = registry_from_handle(registry_handle)?;
        let cfg_str = jstring_to_string(&mut env, &config_json)?;
        let config: AgentConfig = serde_json::from_str(&cfg_str).map_err(|e| {
            JavaBindingError::InvalidArgument(format!("Invalid AgentConfig JSON: {e}"))
        })?;

        let registry_snapshot = reg
            .lock()
            .map_err(|e| JavaBindingError::InvalidArgument(format!("registry poisoned: {e}")))?
            .clone();

        let async_client = (*client.async_client).clone();
        let agent = ToolCallingAgent::new(async_client, registry_snapshot).with_config(config);

        let handle = AgentHandle {
            agent: Arc::new(AsyncMutex::new(agent)),
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
pub extern "system" fn Java_com_liteforge_ToolCallingAgent_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut AgentHandle);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolCallingAgent_nativeRun<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    client_handle: jlong,
    agent_handle: jlong,
    input: JString,
) -> JString<'local> {
    let res = (|| -> Result<String> {
        let client = get_handle(client_handle)?;
        let agent_h = handle_from_ptr(agent_handle)?;
        let input_str = jstring_to_string(&mut env, &input)?;

        let agent = Arc::clone(&agent_h.agent);
        let runtime = Arc::clone(&client.runtime);

        let result = runtime.block_on(async move {
            let mut guard = agent.lock().await;
            guard.run(&input_str).await.map_err(|e| e.to_string())
        });

        result.map_err(JavaBindingError::InvalidArgument)
    })();
    match res {
        Ok(s) => match env.new_string(&s) {
            Ok(js) => js,
            Err(e) => {
                throw_exception(&mut env, e.into());
                JString::default()
            }
        },
        Err(e) => {
            throw_exception(&mut env, e);
            JString::default()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolCallingAgent_nativeRunAsync(
    mut env: JNIEnv,
    _class: JClass,
    client_handle: jlong,
    agent_handle: jlong,
    input: JString,
    callback: JObject,
) {
    let res = (|| -> Result<()> {
        let client = get_handle(client_handle)?;
        let agent_h = handle_from_ptr(agent_handle)?;
        let input_str = jstring_to_string(&mut env, &input)?;

        let cb_ref = env.new_global_ref(callback)?;
        let jvm = Arc::new(env.get_java_vm()?);
        let agent = Arc::clone(&agent_h.agent);
        let runtime = Arc::clone(&client.runtime);

        runtime.spawn(async move {
            let result: std::result::Result<String, String> = {
                let mut guard = agent.lock().await;
                guard.run(&input_str).await.map_err(|e| e.to_string())
            };

            let mut env = match jvm.attach_current_thread() {
                Ok(env) => env,
                Err(_) => return,
            };

            match result {
                Ok(s) => {
                    if let Ok(js) = env.new_string(&s) {
                        let _ = env.call_method(
                            &cb_ref,
                            "onSuccess",
                            "(Ljava/lang/String;)V",
                            &[JValueGen::Object(&js)],
                        );
                    }
                }
                Err(e) => dispatch_err_env(&mut env, &cb_ref, e),
            }
        });
        Ok(())
    })();
    if let Err(e) = res {
        throw_exception(&mut env, e);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolCallingAgent_nativeReset(
    mut env: JNIEnv,
    _class: JClass,
    client_handle: jlong,
    agent_handle: jlong,
) {
    let res = (|| -> Result<()> {
        let client = get_handle(client_handle)?;
        let h = handle_from_ptr(agent_handle)?;
        let agent = Arc::clone(&h.agent);
        client.runtime.block_on(async move {
            let mut guard = agent.lock().await;
            guard.reset();
        });
        Ok(())
    })();
    if let Err(e) = res {
        throw_exception(&mut env, e);
    }
}

fn dispatch_err_env(env: &mut JNIEnv, cb: &jni::objects::GlobalRef, msg: String) {
    if let Ok(js) = env.new_string(&msg) {
        let _ = env.call_method(
            cb,
            "onError",
            "(Ljava/lang/String;)V",
            &[JValueGen::Object(&js)],
        );
    }
}
