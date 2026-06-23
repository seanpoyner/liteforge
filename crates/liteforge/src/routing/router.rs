//! The model router: load-balances across deployments with health-aware
//! selection, cooldowns, model-group fallbacks, and a cross-deployment retry
//! budget. Optionally consults a Layer-2 [`ModelSelector`] to pick which model
//! group(s) a request should target.

use super::config::RouterFileConfig;
use super::deployment::{Deployment, DeploymentId};
use super::health::{now_unix_millis, DeploymentHealth};
use super::selector::{ModelSelector, RouteDecision, SelectionContext};
use super::strategy::{Candidate, RoutingStrategy, SelectionStrategy};
use crate::config::ForgeConfig;
use crate::error::{ForgeError, Result};
use crate::retry::is_retryable;
use crate::transport;
use crate::types::{ChatCompletion, ChatCompletionChunk, ChatCompletionRequest};
use futures::{Stream, StreamExt};
use reqwest::{Client, Method};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Runtime router settings.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RouterSettings {
    /// Load-balancing strategy.
    pub strategy: RoutingStrategy,
    /// Failures before a deployment is cooled down.
    pub allowed_fails: u32,
    /// Cooldown duration applied after `allowed_fails` consecutive failures.
    pub cooldown: Duration,
    /// Additional retry attempts beyond the first, spanning deployments/groups.
    pub num_retries: u32,
    /// EWMA smoothing factor (0..1) for latency-based routing.
    pub ewma_alpha: f64,
    /// When every deployment in every candidate group is cooled down, ignore
    /// cooldowns as a last resort rather than failing.
    pub allow_cooled_fallback: bool,
    /// Model-group fallbacks: group -> ordered fallback groups.
    pub fallbacks: HashMap<String, Vec<String>>,
}

impl Default for RouterSettings {
    fn default() -> Self {
        Self {
            strategy: RoutingStrategy::default(),
            allowed_fails: 3,
            cooldown: Duration::from_secs(60),
            num_retries: 3,
            ewma_alpha: 0.3,
            allow_cooled_fallback: false,
            fallbacks: HashMap::new(),
        }
    }
}

struct RouterInner {
    /// HTTP clients keyed by timeout in milliseconds (so per-deployment
    /// timeouts are honoured without changing the transport layer).
    clients: HashMap<u64, Client>,
    default_client: Client,
    default_timeout_ms: u64,
    base_config: ForgeConfig,
    groups: HashMap<String, Vec<DeploymentId>>,
    deployments: Vec<Deployment>,
    health: Vec<Arc<DeploymentHealth>>,
    strategy: Arc<dyn SelectionStrategy>,
    settings: RouterSettings,
    selector: Option<Arc<dyn ModelSelector>>,
}

/// A model router. Cheap to clone (`Arc` inside); `Send + Sync` and safe to
/// share across many concurrent requests.
#[derive(Clone)]
pub struct Router {
    inner: Arc<RouterInner>,
}

impl Router {
    /// Start building a router programmatically.
    pub fn builder() -> RouterBuilder {
        RouterBuilder::new()
    }

    /// Build a router from a LiteLLM-compatible YAML string.
    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        let cfg = RouterFileConfig::from_yaml_str(yaml)?;
        Self::from_file_config(cfg)
    }

    /// Build a router from a YAML file path.
    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self> {
        let yaml = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            ForgeError::config(format!(
                "could not read router file {:?}: {e}",
                path.as_ref()
            ))
        })?;
        Self::from_yaml_str(&yaml)
    }

    /// Assemble a router from a parsed config file.
    pub fn from_file_config(cfg: RouterFileConfig) -> Result<Self> {
        let deployments = cfg.deployments()?;
        let s = &cfg.router_settings;
        let settings = RouterSettings {
            strategy: s.routing_strategy.unwrap_or_default(),
            allowed_fails: s.allowed_fails.unwrap_or(3),
            cooldown: s.cooldown_time.unwrap_or_else(|| Duration::from_secs(60)),
            num_retries: s.num_retries.unwrap_or(3),
            ewma_alpha: s.ewma_alpha.unwrap_or(0.3),
            allow_cooled_fallback: s.allow_cooled_fallback.unwrap_or(false),
            fallbacks: cfg.fallback_map(),
        };
        let mut builder = RouterBuilder::new().settings(settings);
        builder.deployments = deployments;
        builder.build()
    }

    /// Attach a Layer-2 selector (consumes and returns the router).
    pub fn with_selector(mut self, selector: Arc<dyn ModelSelector>) -> Self {
        // `Arc::make_mut` would clone; instead rebuild the Arc with the
        // selector set. The router is normally configured before sharing.
        if let Some(inner) = Arc::get_mut(&mut self.inner) {
            inner.selector = Some(selector);
            self
        } else {
            // Already shared: clone the inner to attach the selector.
            let new_inner = RouterInner {
                clients: self.inner.clients.clone(),
                default_client: self.inner.default_client.clone(),
                default_timeout_ms: self.inner.default_timeout_ms,
                base_config: self.inner.base_config.clone(),
                groups: self.inner.groups.clone(),
                deployments: self.inner.deployments.clone(),
                health: self.inner.health.clone(),
                strategy: Arc::clone(&self.inner.strategy),
                settings: self.inner.settings.clone(),
                selector: Some(selector),
            };
            Router {
                inner: Arc::new(new_inner),
            }
        }
    }

    /// The model group names this router serves.
    pub fn model_groups(&self) -> Vec<&str> {
        self.inner.groups.keys().map(String::as_str).collect()
    }

    /// The deployment ids registered for a group (empty if unknown).
    pub fn deployments_for(&self, group: &str) -> &[DeploymentId] {
        self.inner
            .groups
            .get(group)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// The selection strategy name.
    pub fn strategy_name(&self) -> &'static str {
        self.inner.strategy.name()
    }

    fn client_for(&self, dep: &Deployment) -> &Client {
        match dep.timeout {
            Some(t) => {
                let key = t.as_millis() as u64;
                self.inner
                    .clients
                    .get(&key)
                    .unwrap_or(&self.inner.default_client)
            }
            None => &self.inner.default_client,
        }
    }

    /// Resolve the ordered list of groups to try (selector ranking + the
    /// primary group's static fallbacks, de-duplicated) and the top score.
    async fn resolve_groups(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<(Vec<String>, Option<f32>)> {
        let (mut ordered, top_score) = match &self.inner.selector {
            Some(sel) => {
                let ctx = SelectionContext::new(req);
                let groups = sel.select(&ctx).await?;
                if groups.is_empty() {
                    (vec![req.model.clone()], None)
                } else {
                    let top = groups.first().map(|g| g.score);
                    (groups.into_iter().map(|g| g.group).collect(), top)
                }
            }
            None => (vec![req.model.clone()], None),
        };

        if let Some(primary) = ordered.first().cloned() {
            if let Some(fbs) = self.inner.settings.fallbacks.get(&primary) {
                ordered.extend(fbs.iter().cloned());
            }
        }

        let mut seen = HashSet::new();
        ordered.retain(|g| seen.insert(g.clone()));
        Ok((ordered, top_score))
    }

    /// Live deployment ids for a group: drops zero-weight and cooled-down
    /// deployments. Falls back to all weighted deployments when everything is
    /// cooled and `allow_cooled_fallback` is set.
    fn live_ids(&self, group: &str, now: u64) -> Vec<DeploymentId> {
        let ids = match self.inner.groups.get(group) {
            Some(v) => v,
            None => return Vec::new(),
        };
        let mut live: Vec<DeploymentId> = ids
            .iter()
            .copied()
            .filter(|id| {
                let d = &self.inner.deployments[id.0];
                d.weight > 0 && !self.inner.health[id.0].is_cooled_down(now)
            })
            .collect();
        if live.is_empty() && self.inner.settings.allow_cooled_fallback {
            live = ids
                .iter()
                .copied()
                .filter(|id| self.inner.deployments[id.0].weight > 0)
                .collect();
        }
        live
    }

    fn pick(&self, live: &[DeploymentId], now: u64) -> usize {
        let cands: Vec<Candidate> = live
            .iter()
            .map(|id| Candidate {
                deployment: &self.inner.deployments[id.0],
                health: self.inner.health[id.0].snapshot(now),
            })
            .collect();
        self.inner.strategy.select(&cands)
    }

    /// Create a chat completion, routing across deployments with fallback.
    pub async fn chat_completions(&self, req: ChatCompletionRequest) -> Result<ChatCompletion> {
        let (ordered, _score) = self.resolve_groups(&req).await?;
        let now = now_unix_millis();
        let mut attempts_left = self.inner.settings.num_retries.saturating_add(1);
        let mut last_error: Option<ForgeError> = None;

        'groups: for group in &ordered {
            let mut live = self.live_ids(group, now);
            if live.is_empty() {
                last_error = Some(ForgeError::ModelNotFound {
                    model: group.clone(),
                    response: None,
                });
                continue;
            }

            while !live.is_empty() {
                if attempts_left == 0 {
                    break 'groups;
                }
                attempts_left -= 1;

                let pick = self.pick(&live, now);
                let chosen_id = live[pick];
                let dep = &self.inner.deployments[chosen_id.0];
                let health = &self.inner.health[chosen_id.0];

                let mut dreq = req.clone();
                dreq.model = dep.model.clone();
                let cfg = dep.to_config(&self.inner.base_config);
                let client = self.client_for(dep);

                let guard = health.on_request_start();
                let started = Instant::now();
                let res: Result<ChatCompletion> = transport::request_with_body(
                    client,
                    &cfg,
                    Method::POST,
                    "/chat/completions",
                    &dreq,
                )
                .await;
                drop(guard);

                match res {
                    Ok(c) => {
                        health.record_success(started.elapsed(), self.inner.settings.ewma_alpha);
                        return Ok(c);
                    }
                    Err(e) => {
                        health.record_failure(
                            self.inner.settings.allowed_fails,
                            self.inner.settings.cooldown,
                            now,
                        );
                        let retryable = is_retryable(&e);
                        last_error = Some(e);
                        live.remove(pick);
                        if !retryable {
                            // Sibling deployments share params; move to the
                            // next group instead of thrashing the budget.
                            continue 'groups;
                        }
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| ForgeError::other("router: no deployments available to route to")))
    }

    /// Create a streaming chat completion. Group/deployment selection and
    /// fallback apply at stream-open time only; once the stream is established,
    /// mid-stream errors surface to the caller (emitted tokens cannot be replayed).
    pub async fn chat_completions_stream(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>> {
        let (ordered, _score) = self.resolve_groups(&req).await?;
        let now = now_unix_millis();
        let mut attempts_left = self.inner.settings.num_retries.saturating_add(1);
        let mut last_error: Option<ForgeError> = None;

        'groups: for group in &ordered {
            let mut live = self.live_ids(group, now);
            if live.is_empty() {
                last_error = Some(ForgeError::ModelNotFound {
                    model: group.clone(),
                    response: None,
                });
                continue;
            }

            while !live.is_empty() {
                if attempts_left == 0 {
                    break 'groups;
                }
                attempts_left -= 1;

                let pick = self.pick(&live, now);
                let chosen_id = live[pick];
                let dep = &self.inner.deployments[chosen_id.0];
                let health = Arc::clone(&self.inner.health[chosen_id.0]);

                let mut dreq = req.clone();
                dreq.model = dep.model.clone();
                dreq.stream = Some(true);
                let cfg = dep.to_config(&self.inner.base_config);
                let client = self.client_for(dep);

                let guard = health.start_owned();
                let started = Instant::now();
                let opened =
                    transport::request_stream(client, &cfg, "/chat/completions", &dreq).await;

                match opened {
                    Ok(stream) => {
                        health.record_success(started.elapsed(), self.inner.settings.ewma_alpha);
                        // Move the guard into the stream so in-flight decrements
                        // when the stream is fully consumed or dropped.
                        let wrapped = async_stream::stream! {
                            let _g = guard;
                            futures::pin_mut!(stream);
                            while let Some(item) = stream.next().await {
                                yield item;
                            }
                        };
                        return Ok(Box::pin(wrapped));
                    }
                    Err(e) => {
                        drop(guard);
                        health.record_failure(
                            self.inner.settings.allowed_fails,
                            self.inner.settings.cooldown,
                            now,
                        );
                        let retryable = is_retryable(&e);
                        last_error = Some(e);
                        live.remove(pick);
                        if !retryable {
                            continue 'groups;
                        }
                    }
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| ForgeError::other("router: no deployments available to route to")))
    }

    /// Resolve the routing decision for a request WITHOUT making the upstream
    /// call. Used by `forge route test` / `which_model` and response headers.
    pub async fn route_decision(&self, req: &ChatCompletionRequest) -> Result<RouteDecision> {
        let (ordered, score) = self.resolve_groups(req).await?;
        let now = now_unix_millis();
        for group in &ordered {
            let mut live = self.live_ids(group, now);
            if live.is_empty() {
                // For introspection, consider all weighted deployments even if cooled.
                if let Some(ids) = self.inner.groups.get(group) {
                    live = ids
                        .iter()
                        .copied()
                        .filter(|id| self.inner.deployments[id.0].weight > 0)
                        .collect();
                }
            }
            if live.is_empty() {
                continue;
            }
            let pick = self.pick(&live, now);
            let dep = &self.inner.deployments[live[pick].0];
            let base_url = if dep.base_url.is_empty() {
                self.inner.base_config.base_url.clone()
            } else {
                dep.base_url.clone()
            };
            return Ok(RouteDecision {
                group: group.clone(),
                model: dep.model.clone(),
                base_url,
                strategy: self.inner.strategy.name().to_string(),
                score,
                fallback_chain: ordered.clone(),
            });
        }
        Err(ForgeError::ModelNotFound {
            model: req.model.clone(),
            response: None,
        })
    }

    /// Convenience: the concrete model id a prompt would route to.
    pub async fn which_model(&self, prompt: impl Into<String>) -> Result<String> {
        let req = ChatCompletionRequest::new("auto", vec![crate::types::Message::user(prompt)]);
        Ok(self.route_decision(&req).await?.model)
    }
}

fn build_http_client(timeout: Duration) -> Client {
    let mut builder = reqwest::Client::builder().timeout(timeout);
    if let Ok(path) = std::env::var("LITEFORGE_EXTRA_CA_FILE") {
        if let Ok(pem) = std::fs::read(&path) {
            if let Ok(certs) = reqwest::Certificate::from_pem_bundle(&pem) {
                for c in certs {
                    builder = builder.add_root_certificate(c);
                }
            }
        }
    }
    builder.build().expect("Failed to build HTTP client")
}

/// Builder for [`Router`].
pub struct RouterBuilder {
    deployments: Vec<Deployment>,
    base_config: Option<ForgeConfig>,
    settings: RouterSettings,
    selector: Option<Arc<dyn ModelSelector>>,
}

impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RouterBuilder {
    /// Create an empty builder.
    pub fn new() -> Self {
        Self {
            deployments: Vec::new(),
            base_config: None,
            settings: RouterSettings::default(),
            selector: None,
        }
    }

    /// Set the base config (provides default api_key/headers/timeout and the
    /// fallback base_url for deployments without an explicit `api_base`).
    pub fn base_config(mut self, config: ForgeConfig) -> Self {
        self.base_config = Some(config);
        self
    }

    /// Replace the router settings.
    pub fn settings(mut self, settings: RouterSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Set the load-balancing strategy.
    pub fn strategy(mut self, strategy: RoutingStrategy) -> Self {
        self.settings.strategy = strategy;
        self
    }

    /// Register a fallback chain for a group.
    pub fn fallback(mut self, group: impl Into<String>, fallbacks: Vec<String>) -> Self {
        self.settings.fallbacks.insert(group.into(), fallbacks);
        self
    }

    /// Set a Layer-2 selector.
    pub fn selector(mut self, selector: Arc<dyn ModelSelector>) -> Self {
        self.selector = Some(selector);
        self
    }

    /// Add a deployment (id assigned automatically).
    pub fn add_deployment(
        mut self,
        group: impl Into<String>,
        model: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        let id = DeploymentId(self.deployments.len());
        self.deployments
            .push(Deployment::new(id, group, model, base_url));
        self
    }

    /// Add a fully-specified deployment (its id is reassigned to keep indices
    /// contiguous and consistent with the health vector).
    pub fn add_deployment_full(mut self, mut deployment: Deployment) -> Self {
        deployment.id = DeploymentId(self.deployments.len());
        self.deployments.push(deployment);
        self
    }

    /// Build the router.
    pub fn build(mut self) -> Result<Router> {
        if self.deployments.is_empty() {
            return Err(ForgeError::config("router has no deployments"));
        }
        // Reassign ids defensively so they are contiguous 0..n.
        for (i, d) in self.deployments.iter_mut().enumerate() {
            d.id = DeploymentId(i);
        }

        let base_config = self.base_config.unwrap_or_else(ForgeConfig::from_env);

        // Group name -> deployment ids.
        let mut groups: HashMap<String, Vec<DeploymentId>> = HashMap::new();
        for d in &self.deployments {
            groups.entry(d.model_group.clone()).or_default().push(d.id);
        }

        // One HTTP client per distinct timeout (base + per-deployment).
        let default_timeout_ms = base_config.timeout.as_millis() as u64;
        let mut clients: HashMap<u64, Client> = HashMap::new();
        clients.insert(default_timeout_ms, build_http_client(base_config.timeout));
        for d in &self.deployments {
            if let Some(t) = d.timeout {
                let key = t.as_millis() as u64;
                clients.entry(key).or_insert_with(|| build_http_client(t));
            }
        }
        let default_client = clients
            .get(&default_timeout_ms)
            .cloned()
            .expect("default client present");

        let health = (0..self.deployments.len())
            .map(|_| Arc::new(DeploymentHealth::new()))
            .collect();

        let strategy = self.settings.strategy.build();

        Ok(Router {
            inner: Arc::new(RouterInner {
                clients,
                default_client,
                default_timeout_ms,
                base_config,
                groups,
                deployments: self.deployments,
                health,
                strategy,
                settings: self.settings,
                selector: self.selector,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::selector::ScoredGroup;
    use async_trait::async_trait;

    fn base() -> ForgeConfig {
        ForgeConfig {
            api_key: Some("k".into()),
            default_model: "m".into(),
            base_url: "https://base/v1".into(),
            timeout: Duration::from_secs(60),
            default_headers: HashMap::new(),
            default_metadata: HashMap::new(),
            otel: None,
        }
    }

    #[test]
    fn builder_requires_deployments() {
        let r = RouterBuilder::new().base_config(base()).build();
        assert!(r.is_err());
    }

    #[test]
    fn builder_groups_deployments() {
        let r = RouterBuilder::new()
            .base_config(base())
            .add_deployment("premium", "gpt", "https://a/v1")
            .add_deployment("premium", "gpt", "https://b/v1")
            .add_deployment("cheap", "haiku", "https://c/v1")
            .build()
            .unwrap();
        assert_eq!(r.deployments_for("premium").len(), 2);
        assert_eq!(r.deployments_for("cheap").len(), 1);
        assert_eq!(r.deployments_for("missing").len(), 0);
    }

    struct FixedSelector(Vec<String>);
    #[async_trait]
    impl ModelSelector for FixedSelector {
        async fn select(&self, _ctx: &SelectionContext<'_>) -> Result<Vec<ScoredGroup>> {
            Ok(self
                .0
                .iter()
                .enumerate()
                .map(|(i, g)| ScoredGroup::new(g, 1.0 - i as f32 * 0.1))
                .collect())
        }
        fn name(&self) -> &str {
            "fixed"
        }
    }

    #[tokio::test]
    async fn route_decision_follows_selector_ranking() {
        let r = RouterBuilder::new()
            .base_config(base())
            .add_deployment("premium", "gpt", "https://a/v1")
            .add_deployment("cheap", "haiku", "https://c/v1")
            .selector(Arc::new(FixedSelector(vec![
                "cheap".into(),
                "premium".into(),
            ])))
            .build()
            .unwrap();
        let req = ChatCompletionRequest::new("auto", vec![crate::types::Message::user("hi")]);
        let dec = r.route_decision(&req).await.unwrap();
        assert_eq!(dec.group, "cheap");
        assert_eq!(dec.model, "haiku");
        assert_eq!(dec.fallback_chain, vec!["cheap", "premium"]);
    }

    #[tokio::test]
    async fn route_decision_without_selector_uses_request_model() {
        let r = RouterBuilder::new()
            .base_config(base())
            .add_deployment("premium", "gpt", "https://a/v1")
            .build()
            .unwrap();
        let req = ChatCompletionRequest::new("premium", vec![crate::types::Message::user("hi")]);
        let dec = r.route_decision(&req).await.unwrap();
        assert_eq!(dec.group, "premium");
        assert_eq!(dec.model, "gpt");
    }
}
