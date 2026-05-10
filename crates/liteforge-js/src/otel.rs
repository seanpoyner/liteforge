//! OTel initialisation helpers exposed to JavaScript.
//!
//! Mirrors `liteforge.init_otel` from the Python binding: idempotent,
//! no-op when the wheel/native module was built without the `otel`
//! feature.

use crate::error::forge_error_to_napi;
use napi::bindgen_prelude::*;
use std::collections::HashMap;
use liteforge::OtelConfig as RustOtelConfig;

/// Initialise the global OTel tracer provider + W3C propagator.
///
/// All fields are optional; omitted values fall back to the matching
/// `OTEL_*` env var. Idempotent, safe to call multiple times. When
/// the native module was built without the `otel` cargo feature, this
/// is a no-op that succeeds silently.
///
/// JS usage:
/// ```javascript
/// const forge = require('@liteforge/sdk');
/// forge.initOtel({
///   endpoint: 'https://irn08782.apps.dynatrace.com/api/v2/otlp/v1/traces',
///   headers: { Authorization: 'Api-Token <token>' },
///   serviceName: 'btsales-agent',
/// });
/// ```
#[napi(object)]
pub struct InitOtelOptions {
    pub endpoint: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub service_name: Option<String>,
    pub resource_attributes: Option<HashMap<String, String>>,
    pub capture_prompts: Option<bool>,
}

#[napi]
pub fn init_otel(options: Option<InitOtelOptions>) -> Result<()> {
    let opts = options.unwrap_or(InitOtelOptions {
        endpoint: None,
        headers: None,
        service_name: None,
        resource_attributes: None,
        capture_prompts: None,
    });

    let cfg = RustOtelConfig {
        endpoint: opts.endpoint,
        headers: opts.headers.unwrap_or_default(),
        service_name: opts.service_name,
        resource_attributes: opts.resource_attributes.unwrap_or_default(),
        capture_prompts: opts.capture_prompts.unwrap_or(false),
    };

    liteforge::init_otel(&cfg).map_err(forge_error_to_napi)
}

/// True when the native module was built with `--features otel`.
/// When false, [`init_otel`] is a no-op.
#[napi]
pub fn otel_feature_enabled() -> bool {
    liteforge::otel_feature_enabled()
}
