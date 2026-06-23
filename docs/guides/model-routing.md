# Model Routing

LiteForge ships a native model router with two composable layers:

- **Layer 1 - Load balancing / reliability** (`routing` feature): one logical model
  name fronts many *deployments*; the router picks one per request, tracks health,
  cools down failing deployments, falls back across deployments and model groups,
  and retries with a budget. This mirrors LiteLLM's `Router`.
- **Layer 2 - Content/quality selection** (`model-routing` feature): a pluggable
  `ModelSelector` decides *which* model group a request should target based on the
  prompt itself - route easy prompts to cheap models and hard ones to strong ones,
  or route by content category.

Both layers are **off by default** and additive. Enable with cargo features:

```toml
liteforge = { version = "*", features = ["model-routing"] }  # implies "routing"
```

## Quick start (CLI)

```bash
# See where a prompt routes (no upstream call is made)
forge route test "prove the Riemann hypothesis sketch" --router examples/router.yaml
forge route test "what time is it?" --router examples/router.yaml --json

# Validate / inspect a config
forge route validate --router examples/router.yaml
forge route list --router examples/router.yaml

# Run an OpenAI-compatible proxy that routes every request
forge serve --router examples/router.yaml
```

## Config (LiteLLM-compatible YAML)

```yaml
model_list:
  - model_name: premium
    litellm_params: { model: claude-opus-4.7, api_base: https://litellm.poyner.ai/v1, api_key: os.environ/LITEFORGE_API_KEY, weight: 1 }
  - model_name: cheap
    litellm_params: { model: claude-haiku-4.5, api_base: https://litellm.poyner.ai/v1, api_key: os.environ/LITEFORGE_API_KEY, weight: 2 }

router_settings:
  routing_strategy: latency-based-routing   # simple-shuffle | round-robin | least-busy | latency-based-routing
  allowed_fails: 3
  cooldown_time: 60s
  num_retries: 3
  fallbacks:
    - premium: [cheap]

model_routing:
  embedding: { base_url: https://litellm.poyner.ai/v1, model: bge-m3, dimensions: 1024 }
  groups:
    - { name: premium, tier: Frontier, models: [claude-opus-4.7] }
    - { name: cheap,   tier: Small,    models: [claude-haiku-4.5] }
  selector:
    kind: mf                       # static | semantic | mf | remote_classifier
    weights_path: ./mf_weights.json
    tier_policy: { thresholds: [0.34, 0.66], direction: higher_is_harder }
    cache: { capacity: 4096, ttl_secs: 300 }
    on_error: static
```

`api_key` / `api_base` support `os.environ/NAME` references. Environment overrides:
`FORGE_ROUTER_CONFIG`, `FORGE_ROUTER_WEIGHTS`, `FORGE_ROUTER_EMBEDDING_BASE_URL`.

## Layer-1 strategies

| Strategy | Behaviour |
|----------|-----------|
| `simple-shuffle` | Weighted random (default). |
| `round-robin` | Cycle through live deployments. |
| `least-busy` | Fewest in-flight requests. |
| `latency-based-routing` | Lowest smoothed (EWMA) latency; unmeasured deployments probed first. |

On failure a deployment's consecutive-failure count grows; after `allowed_fails`
it is cooled down for `cooldown_time` and skipped. Requests then fall over to a
sibling deployment, then to a fallback group, within the `num_retries` budget.

## Layer-2 selectors

- **`static`** - passthrough; use the requested model. Zero overhead.
- **`semantic`** - embed the prompt and match it (cosine) against per-group
  utterance centroids. Natively N-way by content category.
- **`mf`** - a native Rust port of [RouteLLM](https://github.com/lm-sys/RouteLLM)'s
  matrix-factorization router. It embeds the prompt, predicts a scalar "hardness",
  and buckets it across capability tiers (`TierPolicy`) for N-way routing. The MF
  weights are retrained for your embedding model (see below).
- **`remote_classifier`** - call a BERT/causal classifier served behind LiteLLM and
  map its labels to model groups.

All selectors fetch embeddings over HTTP (no local ML inference) and share an
optional decision cache to keep repeated decisions off the network hot path.

### Why MF and not "the BERT router"?

RouteLLM ships four routers. The one named `bert` needs full transformer inference;
their **recommended `mf` router is tiny linear algebra** (an embedding table plus two
small linear layers), which is what LiteForge ports natively. If you want an actual
BERT classifier, serve it behind LiteLLM and use the `remote_classifier` selector.

### Retraining MF for your embedding model

RouteLLM's published MF checkpoint is bound to OpenAI `text-embedding-3-small`
(1536-dim). To use a local embedding model (e.g. `bge-m3`, 1024-dim) the MF weights
must be retrained in that vector space. Use `scripts/retrain_mf.py` on a GPU host,
which embeds RouteLLM's Arena preference data via your LiteLLM gateway, trains the
MF model, and exports `mf_weights.json` in the schema the Rust loader expects. Point
`selector.weights_path` (or `FORGE_ROUTER_WEIGHTS`) at the result. If the weights are
missing, `on_error: static` falls back to passthrough so the service keeps serving.

## SDK usage

=== "Rust"

    ```rust
    use liteforge::routing::Router;
    use liteforge::model_routing::ModelRoutingConfig;
    use std::sync::Arc;

    let yaml = std::fs::read_to_string("router.yaml")?;
    let mut router = Router::from_yaml_str(&yaml)?;
    if let Some(mr) = ModelRoutingConfig::parse_optional(&yaml)? {
        router = router.with_selector(Arc::from(mr.build_selector().await?));
    }
    let decision = router.route_decision(&request).await?;   // introspect
    let completion = router.chat_completions(request).await?; // route + call
    ```

=== "Python"

    ```python
    import liteforge
    router = liteforge.Router.from_file("router.yaml")
    print(await router.which_model("prove this theorem"))
    print(await router.route("prove this theorem"))
    ```

=== "JavaScript"

    ```javascript
    const { Router } = require('liteforge');
    const router = await Router.fromFile('router.yaml');
    console.log(await router.whichModel('prove this theorem'));
    console.log(await router.route('prove this theorem'));
    ```

=== "Java"

    ```java
    try (com.liteforge.Router router = com.liteforge.Router.fromFile("router.yaml")) {
        System.out.println(router.whichModel("prove this theorem"));
        System.out.println(router.route("prove this theorem")); // JSON
    }
    ```
