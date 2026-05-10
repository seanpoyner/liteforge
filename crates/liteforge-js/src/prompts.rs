use napi::bindgen_prelude::*;
use std::collections::HashMap;
use liteforge::prompts::{
    CommonPrompts as RustCommonPrompts, PromptConfig as RustPromptConfig,
    PromptLibrary as RustPromptLibrary, PromptTemplate as RustPromptTemplate,
};

#[napi]
pub struct PromptTemplate {
    inner: RustPromptTemplate,
}

#[napi]
impl PromptTemplate {
    #[napi(constructor)]
    pub fn new(template: String) -> Self {
        Self {
            inner: RustPromptTemplate::new(template),
        }
    }

    #[napi]
    pub fn render(&self, vars: HashMap<String, String>) -> Result<String> {
        self.inner
            .render(&vars)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn variables(&self) -> Vec<String> {
        self.inner.variables()
    }
}

#[napi]
pub struct PromptLibrary {
    inner: RustPromptLibrary,
}

#[napi]
impl PromptLibrary {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: RustPromptLibrary::new(),
        }
    }

    #[napi]
    pub fn add(&mut self, name: String, template: String) {
        let t = RustPromptTemplate::new(template);
        self.inner.add(name, t);
    }

    #[napi]
    pub fn add_with_category(&mut self, name: String, template: String, category: String) {
        let t = RustPromptTemplate::new(template);
        self.inner.add_with_category(name, t, category);
    }

    #[napi]
    pub fn render(&self, name: String, vars: HashMap<String, String>) -> Result<String> {
        self.inner
            .render(&name, &vars)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn has(&self, name: String) -> bool {
        self.inner.has(&name)
    }

    #[napi]
    pub fn list(&self) -> Vec<String> {
        self.inner
            .list()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[napi]
    pub fn categories(&self) -> Vec<String> {
        self.inner
            .categories()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[napi]
    pub fn list_by_category(&self, category: String) -> Vec<String> {
        self.inner
            .list_by_category(&category)
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[napi]
    pub fn remove(&mut self, name: String) -> bool {
        self.inner.remove(&name).is_some()
    }

    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    #[napi]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[napi]
pub struct JsPromptConfig {
    inner: RustPromptConfig,
}

#[napi]
impl JsPromptConfig {
    #[napi(constructor)]
    pub fn new(name: String, template: String) -> Self {
        Self {
            inner: RustPromptConfig::new(name, template),
        }
    }

    #[napi]
    pub fn description(&mut self, desc: String) -> &Self {
        self.inner = self.inner.clone().with_description(desc);
        self
    }

    #[napi]
    pub fn category(&mut self, cat: String) -> &Self {
        self.inner = self.inner.clone().with_category(cat);
        self
    }

    #[napi]
    pub fn default_var(&mut self, key: String, value: String) -> &Self {
        self.inner = self.inner.clone().with_default(key, value);
        self
    }

    #[napi]
    pub fn tag(&mut self, tag: String) -> &Self {
        self.inner = self.inner.clone().with_tag(tag);
        self
    }
}

#[napi]
pub struct CommonPrompts {}

#[napi]
impl CommonPrompts {
    #[napi(factory)]
    pub fn summarize() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::summarize(),
        }
    }

    #[napi(factory)]
    pub fn translate() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::translate(),
        }
    }

    #[napi(factory)]
    pub fn qa() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::qa(),
        }
    }

    #[napi(factory)]
    pub fn code_review() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::code_review(),
        }
    }

    #[napi(factory)]
    pub fn classify() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::classify(),
        }
    }

    #[napi(factory)]
    pub fn extract_entities() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::extract_entities(),
        }
    }

    #[napi(factory)]
    pub fn rewrite() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::rewrite(),
        }
    }

    #[napi(factory)]
    pub fn chain_of_thought() -> PromptTemplate {
        PromptTemplate {
            inner: RustCommonPrompts::chain_of_thought(),
        }
    }

    #[napi(factory)]
    pub fn library() -> PromptLibrary {
        PromptLibrary {
            inner: RustCommonPrompts::library(),
        }
    }
}
