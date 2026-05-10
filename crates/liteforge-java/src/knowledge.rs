//! JNI bindings for the local knowledge backend (in-memory RAG store).
//!
//! Design:
//! - The Java side constructs `Document`s, serializes a batch to a JSON
//!   array, and passes it to `upload`.
//! - Search returns a JSON array of `SearchResult`s, which Java parses.
//! - All methods block on the shared `ForgeClient` runtime.

use crate::client::get_handle;
use crate::error::{throw_exception, JavaBindingError, Result};
use crate::types::jstring_to_string;
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jlong, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use std::sync::Arc;
use liteforge::knowledge::{
    Document, KnowledgeClient, ListOptions, LocalKnowledgeBackend, SearchOptions,
};

pub(crate) struct KnowledgeHandle {
    backend: Arc<LocalKnowledgeBackend>,
    runtime: Arc<tokio::runtime::Runtime>,
}

fn handle_from_ptr(ptr: jlong) -> Result<&'static KnowledgeHandle> {
    if ptr == 0 {
        return Err(JavaBindingError::NullPointer(
            "LocalKnowledgeBackend handle is null".into(),
        ));
    }
    Ok(unsafe { &*(ptr as *const KnowledgeHandle) })
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_LocalKnowledgeBackend_nativeCreate(
    mut env: JNIEnv,
    _class: JClass,
    client_handle: jlong,
) -> jlong {
    let res = (|| -> Result<jlong> {
        let client = get_handle(client_handle)?;
        let handle = KnowledgeHandle {
            backend: Arc::new(LocalKnowledgeBackend::new()),
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
pub extern "system" fn Java_com_liteforge_LocalKnowledgeBackend_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut KnowledgeHandle);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_liteforge_LocalKnowledgeBackend_nativeUpload(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    docs_json: JString,
) -> jint {
    let res = (|| -> Result<jint> {
        let h = handle_from_ptr(handle)?;
        let docs_str = jstring_to_string(&mut env, &docs_json)?;
        let docs: Vec<Document> = serde_json::from_str(&docs_str).map_err(|e| {
            JavaBindingError::InvalidArgument(format!("Invalid documents JSON array: {e}"))
        })?;

        let backend = Arc::clone(&h.backend);
        let ids = h
            .runtime
            .block_on(async move { backend.upload(docs).await })
            .map_err(|e| JavaBindingError::InvalidArgument(format!("upload: {e}")))?;
        Ok(ids.len() as jint)
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
pub extern "system" fn Java_com_liteforge_LocalKnowledgeBackend_nativeSearch<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
    query: JString,
    limit: jint,
    namespace: JString,
) -> JString<'local> {
    let res = (|| -> Result<JString<'local>> {
        let h = handle_from_ptr(handle)?;
        let q = jstring_to_string(&mut env, &query)?;
        let mut opts = SearchOptions::new();
        if limit > 0 {
            opts = opts.limit(limit as usize);
        }
        if !namespace.is_null() {
            let ns = jstring_to_string(&mut env, &namespace)?;
            if !ns.is_empty() {
                opts = opts.namespace(ns);
            }
        }

        let backend = Arc::clone(&h.backend);
        let results = h
            .runtime
            .block_on(async move { backend.search(&q, opts).await })
            .map_err(|e| JavaBindingError::InvalidArgument(format!("search: {e}")))?;

        let json = serde_json::to_string(&results)
            .map_err(|e| JavaBindingError::InvalidArgument(format!("serialize: {e}")))?;
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
pub extern "system" fn Java_com_liteforge_LocalKnowledgeBackend_nativeGet<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
    id: JString,
) -> JString<'local> {
    let res = (|| -> Result<JString<'local>> {
        let h = handle_from_ptr(handle)?;
        let id_str = jstring_to_string(&mut env, &id)?;
        let backend = Arc::clone(&h.backend);
        let doc = h
            .runtime
            .block_on(async move { backend.get(&id_str).await })
            .map_err(|e| JavaBindingError::InvalidArgument(format!("get: {e}")))?;

        match doc {
            Some(d) => {
                let json = serde_json::to_string(&d).map_err(|e| {
                    JavaBindingError::InvalidArgument(format!("serialize doc: {e}"))
                })?;
                Ok(env.new_string(&json)?)
            }
            None => Ok(JString::default()),
        }
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
pub extern "system" fn Java_com_liteforge_LocalKnowledgeBackend_nativeDelete(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    id: JString,
) -> jboolean {
    let res = (|| -> Result<bool> {
        let h = handle_from_ptr(handle)?;
        let id_str = jstring_to_string(&mut env, &id)?;
        let backend = Arc::clone(&h.backend);
        let deleted = h
            .runtime
            .block_on(async move { backend.delete(&id_str).await })
            .map_err(|e| JavaBindingError::InvalidArgument(format!("delete: {e}")))?;
        Ok(deleted)
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
pub extern "system" fn Java_com_liteforge_LocalKnowledgeBackend_nativeStatsJson<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
) -> JString<'local> {
    let res = (|| -> Result<JString<'local>> {
        let h = handle_from_ptr(handle)?;
        let backend = Arc::clone(&h.backend);
        let stats = h
            .runtime
            .block_on(async move { backend.stats().await })
            .map_err(|e| JavaBindingError::InvalidArgument(format!("stats: {e}")))?;
        let json = serde_json::to_string(&stats)
            .map_err(|e| JavaBindingError::InvalidArgument(format!("serialize stats: {e}")))?;
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
pub extern "system" fn Java_com_liteforge_LocalKnowledgeBackend_nativeListJson<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass,
    handle: jlong,
    limit: jint,
    offset: jint,
    namespace: JString,
) -> JString<'local> {
    let res = (|| -> Result<JString<'local>> {
        let h = handle_from_ptr(handle)?;
        let mut opts = ListOptions::new();
        if limit > 0 {
            opts = opts.limit(limit as usize);
        }
        if offset > 0 {
            opts = opts.offset(offset as usize);
        }
        if !namespace.is_null() {
            let ns = jstring_to_string(&mut env, &namespace)?;
            if !ns.is_empty() {
                opts = opts.namespace(ns);
            }
        }

        let backend = Arc::clone(&h.backend);
        let docs = h
            .runtime
            .block_on(async move { backend.list(opts).await })
            .map_err(|e| JavaBindingError::InvalidArgument(format!("list: {e}")))?;
        let json = serde_json::to_string(&docs)
            .map_err(|e| JavaBindingError::InvalidArgument(format!("serialize docs: {e}")))?;
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
