//! Python bindings for native model routing (config-driven).
//!
//! Custom selectors are not implemented across the FFI boundary; users choose a
//! built-in selector via the YAML config. This exposes the `Router` with async
//! `route` / `which_model` / `chat_completions` plus simple introspection.

use crate::{completion_to_dict, list_to_messages};
use liteforge_core::model_routing::ModelRoutingConfig;
use liteforge_core::routing::Router as CoreRouter;
use liteforge_core::{ChatCompletionRequest, Message as RustMessage};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn rt_err<E: std::fmt::Display>(e: E) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
}

/// A model router built from a LiteLLM-compatible YAML config.
#[pyclass]
pub struct Router {
    inner: Arc<CoreRouter>,
}

fn build_router(yaml: &str) -> PyResult<Router> {
    let mut router = CoreRouter::from_yaml_str(yaml).map_err(rt_err)?;
    if let Some(mr) = ModelRoutingConfig::parse_optional(yaml).map_err(rt_err)? {
        // Building a selector may embed utterances (semantic) or load weights
        // (MF); run it to completion on a temporary runtime.
        let runtime = Runtime::new().map_err(rt_err)?;
        let selector = runtime.block_on(mr.build_selector()).map_err(rt_err)?;
        router = router.with_selector(Arc::from(selector));
    }
    Ok(Router {
        inner: Arc::new(router),
    })
}

#[pymethods]
impl Router {
    /// Build a router from a YAML string.
    #[staticmethod]
    fn from_yaml(yaml: String) -> PyResult<Self> {
        build_router(&yaml)
    }

    /// Build a router from a YAML file path.
    #[staticmethod]
    fn from_file(path: String) -> PyResult<Self> {
        let yaml = std::fs::read_to_string(&path).map_err(rt_err)?;
        build_router(&yaml)
    }

    /// The concrete model id a prompt would route to (async -> str).
    fn which_model<'py>(&self, py: Python<'py>, prompt: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            inner.which_model(prompt).await.map_err(rt_err)
        })
    }

    /// The full routing decision for a prompt (async -> dict).
    fn route<'py>(&self, py: Python<'py>, prompt: String) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let req = ChatCompletionRequest::new("auto", vec![RustMessage::user(prompt)]);
            let d = inner.route_decision(&req).await.map_err(rt_err)?;
            Python::with_gil(|py| -> PyResult<PyObject> {
                let dict = PyDict::new_bound(py);
                dict.set_item("group", &d.group)?;
                dict.set_item("model", &d.model)?;
                dict.set_item("base_url", &d.base_url)?;
                dict.set_item("strategy", &d.strategy)?;
                dict.set_item("score", d.score)?;
                dict.set_item("fallback_chain", d.fallback_chain)?;
                Ok(dict.into())
            })
        })
    }

    /// Route a chat completion through the router (async -> dict).
    #[pyo3(signature = (messages, model=None, temperature=None, max_tokens=None))]
    fn chat_completions<'py>(
        &self,
        py: Python<'py>,
        messages: &Bound<'py, PyList>,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let msgs = list_to_messages(messages)?;
        let model = model.unwrap_or_else(|| "auto".to_string());
        let mut req = ChatCompletionRequest::new(model, msgs);
        if let Some(t) = temperature {
            req = req.temperature(t);
        }
        if let Some(m) = max_tokens {
            req = req.max_tokens(m);
        }
        let inner = self.inner.clone();
        pyo3_asyncio_0_21::tokio::future_into_py(py, async move {
            let result = inner.chat_completions(req).await.map_err(rt_err)?;
            Python::with_gil(|py| completion_to_dict(py, result))
        })
    }

    /// The model group names this router serves.
    fn model_groups(&self) -> Vec<String> {
        self.inner
            .model_groups()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// The load-balancing strategy name.
    fn strategy(&self) -> String {
        self.inner.strategy_name().to_string()
    }
}
