# Model Routing

LiteForge has a native model router with two composable, feature-gated layers
(both off by default; enable with the `model-routing` cargo feature, which implies
`routing`).

## Layer 1: load balancing and reliability

One logical model name (a "model group") fronts many deployments. The router picks
a deployment per request, tracks per-deployment health, cools down failing ones,
and falls back across deployments and groups within a retry budget. This mirrors
LiteLLM's `Router`.

Strategies: `simple-shuffle` (weighted, default), `round-robin`, `least-busy`,
`latency-based-routing`.

## Layer 2: content and quality selection

A pluggable `ModelSelector` chooses which model group a request targets:

- `static`: passthrough.
- `semantic`: embed the prompt, match (cosine) against per-group utterance
  centroids. N-way by content category.
- `mf`: native Rust port of RouteLLM's matrix-factorization quality router. Predicts
  prompt difficulty and buckets it across capability tiers. The recommended RouteLLM
  router (not the heavyweight `bert` one).
- `remote_classifier`: call a BERT/causal classifier served behind LiteLLM and map
  labels to groups.

Embeddings are fetched over HTTP (no local ML inference). A decision cache keeps
repeated decisions off the hot path.

## Config

LiteLLM-compatible YAML: `model_list` + `router_settings` (Layer 1) plus a
`model_routing` block (Layer 2). See `examples/router.yaml` in the repo and the
[SDK docs](https://your-docs-site/guides/model-routing/) for the full schema.

Env overrides: `FORGE_ROUTER_CONFIG`, `FORGE_ROUTER_WEIGHTS`,
`FORGE_ROUTER_EMBEDDING_BASE_URL`.

## CLI

```bash
forge route test "prove this theorem" --router examples/router.yaml
forge route validate --router examples/router.yaml
forge route list --router examples/router.yaml
forge serve --router examples/router.yaml   # OpenAI-compatible routing proxy
```

## MF retraining

RouteLLM's published MF weights are bound to OpenAI `text-embedding-3-small`. To use
a local embedding model (e.g. `bge-m3`), retrain MF in that vector space with
`scripts/retrain_mf.py` on a GPU host, then point `selector.weights_path` (or
`FORGE_ROUTER_WEIGHTS`) at the exported `mf_weights.json`. If weights are missing,
`on_error: static` keeps the service serving via passthrough.

## Bindings

`Router` is exposed in Python, JavaScript, and Java (config-driven via YAML):
`from_yaml` / `from_file`, `route` / `which_model`, `model_groups`, `strategy`.
