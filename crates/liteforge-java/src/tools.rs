//! JNI bindings for ToolRegistry, ToolExecutor, and Java-defined tools.
//!
//! Design:
//! - A Java `Tool` interface is passed into `ToolRegistry.register()`. The JNI
//!   layer pins a `GlobalRef` to that object and captures the `JavaVM`. A
//!   Rust-side `JavaCallableTool` implements `liteforge::tools::Tool` by calling
//!   back into Java through `JNIEnv::attach_current_thread` when the agent or
//!   executor invokes it.
//! - JSON crosses the FFI boundary as `String`. Java callers parse it with
//!   their library of choice (Jackson, Gson, org.json, etc.).

use crate::error::{throw_exception, JavaBindingError, Result};
use crate::types::jstring_to_string;
use jni::objects::{GlobalRef, JClass, JObject, JString, JValueGen};
use jni::sys::{jboolean, jint, jlong, JNI_FALSE, JNI_TRUE};
use jni::{JNIEnv, JavaVM};
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};
use liteforge::tools::{
    Tool as RustTool, ToolExecutor as RustToolExecutor, ToolRegistry as RustToolRegistry,
    ToolResult as RustToolResult,
};

// ---------------------------------------------------------------------------
// JavaCallableTool: wraps a Java `Tool` as a Rust `Tool`.
// ---------------------------------------------------------------------------

pub(crate) struct JavaCallableTool {
    name: String,
    description: String,
    parameters: JsonValue,
    callback: Arc<GlobalRef>,
    jvm: Arc<JavaVM>,
    requires_confirmation: bool,
}

impl RustTool for JavaCallableTool {
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
        let mut env = self
            .jvm
            .attach_current_thread()
            .map_err(|e| format!("Failed to attach JVM thread: {e}"))?;

        let args_str =
            serde_json::to_string(&args).map_err(|e| format!("Failed to serialize args: {e}"))?;
        let args_jstr = env
            .new_string(&args_str)
            .map_err(|e| format!("new_string: {e}"))?;

        let call_result = env.call_method(
            self.callback.as_obj(),
            "execute",
            "(Ljava/lang/String;)Ljava/lang/String;",
            &[JValueGen::Object(&args_jstr)],
        );

        // Surface Java exceptions as Rust errors.
        if env.exception_check().unwrap_or(false) {
            let msg =
                exception_message(&mut env).unwrap_or_else(|| "unknown Java exception".to_string());
            let _ = env.exception_clear();
            return Err(format!("Java tool threw: {msg}"));
        }

        let result_obj = call_result
            .map_err(|e| format!("Tool.execute call failed: {e}"))?
            .l()
            .map_err(|e| format!("Tool.execute did not return an Object: {e}"))?;

        if result_obj.is_null() {
            return Ok(JsonValue::Null);
        }

        let jstr = JString::from(result_obj);
        let s: String = env
            .get_string(&jstr)
            .map_err(|e| format!("get_string: {e}"))?
            .into();

        serde_json::from_str(&s).map_err(|e| format!("Result JSON parse failed: {e}"))
    }

    fn requires_confirmation(&self) -> bool {
        self.requires_confirmation
    }
}

fn exception_message(env: &mut JNIEnv) -> Option<String> {
    let exc = env.exception_occurred().ok()?;
    env.exception_clear().ok()?;
    let msg_obj = env
        .call_method(&exc, "getMessage", "()Ljava/lang/String;", &[])
        .ok()?
        .l()
        .ok()?;
    if msg_obj.is_null() {
        return None;
    }
    let jstr = JString::from(msg_obj);
    let s: String = env.get_string(&jstr).ok()?.into();
    Some(s)
}

// ---------------------------------------------------------------------------
// ToolRegistry handle: Arc<Mutex<RustToolRegistry>> so agents/executors can
// share a registry and mutate it from any Java thread.
// ---------------------------------------------------------------------------

pub(crate) type SharedRegistry = Arc<Mutex<RustToolRegistry>>;

pub(crate) fn registry_from_handle(ptr: jlong) -> Result<SharedRegistry> {
    if ptr == 0 {
        return Err(JavaBindingError::NullPointer(
            "ToolRegistry handle is null".into(),
        ));
    }
    let arc = unsafe { &*(ptr as *const SharedRegistry) };
    Ok(arc.clone())
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolRegistry_nativeCreate(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let reg: SharedRegistry = Arc::new(Mutex::new(RustToolRegistry::new()));
    Box::into_raw(Box::new(reg)) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolRegistry_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut SharedRegistry);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolRegistry_nativeRegister(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    tool: JObject,
) {
    let res = (|| -> Result<()> {
        let reg = registry_from_handle(handle)?;

        let name = call_string_getter(&mut env, &tool, "name")?;
        let description = call_string_getter(&mut env, &tool, "description")?;
        let params_str = call_string_getter(&mut env, &tool, "parametersSchemaJson")?;
        let parameters: JsonValue = serde_json::from_str(&params_str).map_err(|e| {
            JavaBindingError::InvalidArgument(format!("Invalid parameters JSON: {e}"))
        })?;
        let requires_confirmation = call_bool_getter(&mut env, &tool, "requiresConfirmation")?;

        let global_ref = env.new_global_ref(&tool)?;
        let jvm = env.get_java_vm()?;

        let rust_tool = JavaCallableTool {
            name,
            description,
            parameters,
            callback: Arc::new(global_ref),
            jvm: Arc::new(jvm),
            requires_confirmation,
        };

        reg.lock()
            .map_err(|e| JavaBindingError::InvalidArgument(format!("registry poisoned: {e}")))?
            .register(Box::new(rust_tool));
        Ok(())
    })();

    if let Err(e) = res {
        throw_exception(&mut env, e);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolRegistry_nativeUnregister(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    name: JString,
) -> jboolean {
    let res = (|| -> Result<bool> {
        let reg = registry_from_handle(handle)?;
        let n = jstring_to_string(&mut env, &name)?;
        let found = reg
            .lock()
            .map_err(|e| JavaBindingError::InvalidArgument(format!("registry poisoned: {e}")))?
            .unregister(&n)
            .is_some();
        Ok(found)
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
pub extern "system" fn Java_com_liteforge_ToolRegistry_nativeContains(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    name: JString,
) -> jboolean {
    let res = (|| -> Result<bool> {
        let reg = registry_from_handle(handle)?;
        let n = jstring_to_string(&mut env, &name)?;
        let found = reg
            .lock()
            .map_err(|e| JavaBindingError::InvalidArgument(format!("registry poisoned: {e}")))?
            .contains(&n);
        Ok(found)
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
pub extern "system" fn Java_com_liteforge_ToolRegistry_nativeSize(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    let res = (|| -> Result<jint> {
        let reg = registry_from_handle(handle)?;
        let n = reg
            .lock()
            .map_err(|e| JavaBindingError::InvalidArgument(format!("registry poisoned: {e}")))?
            .len() as jint;
        Ok(n)
    })();
    match res {
        Ok(n) => n,
        Err(e) => {
            throw_exception(&mut env, e);
            0
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolRegistry_nativeNames<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
) -> JObject<'local> {
    let res = (|| -> Result<JObject<'local>> {
        let reg = registry_from_handle(handle)?;
        let names: Vec<String> = reg
            .lock()
            .map_err(|e| JavaBindingError::InvalidArgument(format!("registry poisoned: {e}")))?
            .names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        let array_list_class = env.find_class("java/util/ArrayList")?;
        let list = env.new_object(array_list_class, "()V", &[])?;
        for name in names {
            let js = env.new_string(&name)?;
            env.call_method(
                &list,
                "add",
                "(Ljava/lang/Object;)Z",
                &[JValueGen::Object(&js)],
            )?;
        }
        Ok(list)
    })();
    match res {
        Ok(o) => o,
        Err(e) => {
            throw_exception(&mut env, e);
            JObject::null()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolRegistry_nativeDefinitionsJson<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
) -> JString<'local> {
    let res = (|| -> Result<JString<'local>> {
        let reg = registry_from_handle(handle)?;
        let defs = reg
            .lock()
            .map_err(|e| JavaBindingError::InvalidArgument(format!("registry poisoned: {e}")))?
            .definitions();
        let json = serde_json::to_string(&defs)
            .map_err(|e| JavaBindingError::InvalidArgument(format!("Serialize defs: {e}")))?;
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

// ---------------------------------------------------------------------------
// ToolExecutor: shares the registry handle, executes by name.
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolExecutor_nativeCreate(
    mut env: JNIEnv,
    _class: JClass,
    registry_handle: jlong,
) -> jlong {
    if registry_handle == 0 {
        throw_exception(
            &mut env,
            JavaBindingError::NullPointer("ToolRegistry handle is null".into()),
        );
        return 0;
    }
    let reg = unsafe { &*(registry_handle as *const SharedRegistry) }.clone();
    Box::into_raw(Box::new(reg)) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolExecutor_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut SharedRegistry);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolExecutor_nativeExecute<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
    name: JString,
    args_json: JString,
) -> JObject<'local> {
    let res = (|| -> Result<JObject<'local>> {
        let reg = registry_from_handle(handle)?;
        let name_str = jstring_to_string(&mut env, &name)?;
        let args_str = jstring_to_string(&mut env, &args_json)?;
        let args: JsonValue = serde_json::from_str(&args_str)
            .map_err(|e| JavaBindingError::InvalidArgument(format!("Invalid args JSON: {e}")))?;

        // `ToolExecutor` takes a `ToolRegistry` by value; the core registry is
        // `Clone` and tools are `Arc<dyn Tool>`, so cloning is cheap.
        let registry_snapshot = reg
            .lock()
            .map_err(|e| JavaBindingError::InvalidArgument(format!("registry poisoned: {e}")))?
            .clone();
        let executor = RustToolExecutor::new(registry_snapshot);
        let result = executor.execute(&name_str, args);

        tool_result_to_jobject(&mut env, result)
    })();
    match res {
        Ok(o) => o,
        Err(e) => {
            throw_exception(&mut env, e);
            JObject::null()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_ToolExecutor_nativeExecuteWithId<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
    call_id: JString,
    name: JString,
    args_json: JString,
) -> JObject<'local> {
    let res = (|| -> Result<JObject<'local>> {
        let reg = registry_from_handle(handle)?;
        let id_str = jstring_to_string(&mut env, &call_id)?;
        let name_str = jstring_to_string(&mut env, &name)?;
        let args_str = jstring_to_string(&mut env, &args_json)?;
        let args: JsonValue = serde_json::from_str(&args_str)
            .map_err(|e| JavaBindingError::InvalidArgument(format!("Invalid args JSON: {e}")))?;

        let registry_snapshot = reg
            .lock()
            .map_err(|e| JavaBindingError::InvalidArgument(format!("registry poisoned: {e}")))?
            .clone();
        let executor = RustToolExecutor::new(registry_snapshot);
        let result = executor.execute_with_id(&id_str, &name_str, args);

        tool_result_to_jobject(&mut env, result)
    })();
    match res {
        Ok(o) => o,
        Err(e) => {
            throw_exception(&mut env, e);
            JObject::null()
        }
    }
}

pub(crate) fn tool_result_to_jobject<'local>(
    env: &mut JNIEnv<'local>,
    r: RustToolResult,
) -> Result<JObject<'local>> {
    let class = env.find_class("com/liteforge/ToolResult")?;
    let tool_call_id = env.new_string(&r.tool_call_id)?;
    let name = env.new_string(&r.name)?;
    let success = if r.success { JNI_TRUE } else { JNI_FALSE };

    let result_json = if let Some(v) = &r.result {
        let s = serde_json::to_string(v).unwrap_or_else(|_| "null".to_string());
        JObject::from(env.new_string(&s)?)
    } else {
        JObject::null()
    };

    let error = if let Some(e) = &r.error {
        JObject::from(env.new_string(e)?)
    } else {
        JObject::null()
    };

    let exec_time = if let Some(ms) = r.execution_time_ms {
        let long_class = env.find_class("java/lang/Long")?;
        env.new_object(long_class, "(J)V", &[JValueGen::Long(ms as i64)])?
    } else {
        JObject::null()
    };

    let obj = env.new_object(
        class,
        "(Ljava/lang/String;Ljava/lang/String;ZLjava/lang/String;Ljava/lang/String;Ljava/lang/Long;)V",
        &[
            JValueGen::Object(&tool_call_id),
            JValueGen::Object(&name),
            JValueGen::Bool(success),
            JValueGen::Object(&result_json),
            JValueGen::Object(&error),
            JValueGen::Object(&exec_time),
        ],
    )?;
    Ok(obj)
}

// ---------------------------------------------------------------------------
// Helpers for reading values off Java `Tool` instances.
// ---------------------------------------------------------------------------

fn call_string_getter(env: &mut JNIEnv, obj: &JObject, method: &str) -> Result<String> {
    let result_obj = env
        .call_method(obj, method, "()Ljava/lang/String;", &[])?
        .l()?;
    if result_obj.is_null() {
        return Err(JavaBindingError::InvalidArgument(format!(
            "Tool.{method}() returned null"
        )));
    }
    let jstr = JString::from(result_obj);
    let s: String = env.get_string(&jstr)?.into();
    Ok(s)
}

fn call_bool_getter(env: &mut JNIEnv, obj: &JObject, method: &str) -> Result<bool> {
    let result = env.call_method(obj, method, "()Z", &[])?.z()?;
    Ok(result)
}
