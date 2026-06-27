# Cross-provider cost-quality and build-vs-buy study

Status: FINAL - all 25 models measured (n=290 public-benchmark prompts). Hosted/Claude
models are priced from the gateway's measured cost; the open-weight (OSS) models were
quality-measured via Ollama Cloud and are priced at their published serverless rates (Together /
DeepInfra / Mistral). Two Qwen cloud models are truncation artifacts (excluded from conclusions;
see notes).

Author: Sean Poyner. Date: 2026-06-25 (cost column corrected to published rates 2026-06-27).
Figures: `results/figures/provider_*.png`.

## TL;DR

1. **The biggest cost win is changing the default model, and it needs no router.** On a
   public benchmark pool that genuinely separates the models:
   - **DeepSeek v4 Flash: 95% of Sonnet quality at $0.26 / 1M tokens** (vs Sonnet $11.27).
     That is ~43x cheaper at a 4.6-point quality gap.
   - **Gemini 3.1 Flash-Lite: 96% of Sonnet at $1.05 / 1M** if you want the highest cheap
     tier.
   - Claude Haiku is Pareto-dominated by both (lower quality, higher price).
2. **Routing does not earn its keep here.** A bge-m3 head choosing between the cheap default
   and Sonnet showed no gain over just defaulting cheap (APGR 0.004, 95% CI [-0.41, 0.39]).
   The cheap models are close enough that there is little quality gap to recover.
3. **The strongest OSS reaches ~96% of Sonnet but none beats it, and all are large.**
   gemma4:31b (0.865), glm-5.2 (0.861), deepseek-v4-pro (0.854), and gpt-oss-120b (0.844)
   all land at 94-96% of Sonnet - genuinely close - but they are 31B-to-very-large models,
   expensive to self-host, so cheap hosted (Gemini Flash-Lite, DeepSeek Flash) still wins on
   cost-quality below very high volume. The watsonx-class small model (Granite-8B, 0.735) is
   Pareto-dominated by cheaper hosted Mistral Small.

Recommendation: default general traffic to a cheap hosted frontier model (DeepSeek Flash or
Gemini Flash-Lite; in-tenant, the Vertex/Bedrock equivalents), keep Sonnet for the
hard-reasoning tail, and skip the ML router. Treat self-host/watsonx as a high-volume or
data-residency play, not a unit-cost play.

## Cost-quality frontier (n=290, interim 14 models)

| Model | Path | Accuracy (95% CI) | % of Sonnet | $/1M tokens |
|---|---|---|---|---|
| Sonnet 4.6 | anchor | 0.896 [0.86, 0.93] | 100% | $11.27 (measured) |
| **gemma4:31b** | OSS | 0.865 [0.82, 0.90] | 96% | **$0.78 (published)** |
| glm-5.2 | OSS | 0.861 [0.82, 0.90] | 96% | $4.08 (published) |
| minimax-m3 | OSS | 0.861 [0.82, 0.90] | 96% | $1.10 (published) |
| Gemini 3.1 Flash-Lite | hosted | 0.858 [0.82, 0.90] | 96% | $1.05 (measured) |
| deepseek-v4-pro | OSS | 0.854 [0.81, 0.89] | 95% | $3.31 (published) |
| **DeepSeek v4 Flash** | hosted | 0.850 [0.81, 0.89] | **95%** | **$0.26 (measured)** |
| **gpt-oss-120b** | OSS | 0.844 [0.80, 0.89] | 94% | **$0.49 (published)** |
| qwen3-coder-480b | OSS | 0.841 [0.79, 0.88] | 94% | $0.79 (published) |
| Haiku 4.5 | anchor | 0.836 [0.79, 0.88] | 93% | $3.86 (measured) |
| devstral-24b | OSS | 0.824 [0.78, 0.87] | 92% | $0.23 (published) |
| nemotron-ultra | OSS | 0.817 [0.77, 0.86] | 91% | $3.36 (published) |
| kimi-k2.7 | OSS | 0.817 [0.77, 0.86] | 91% | $3.87 (published) |
| gpt-oss-20b | OSS | 0.792 [0.75, 0.84] | 88% | $0.17 (published) |
| gemma3-27b | OSS | 0.771 [0.72, 0.82] | 86% | $0.14 (published) |
| ministral-14b | OSS | 0.762 [0.71, 0.81] | 85% | $0.20 (published) |
| nemotron-nano-30b | OSS | 0.761 [0.71, 0.81] | 85% | $0.19 (published) |
| Mistral Small | hosted | 0.741 [0.69, 0.79] | 83% | $0.09 (measured) |
| Granite 4.1 8B | self-host | 0.735 [0.68, 0.79] | 82% | $0.19 (watsonx) |
| ministral-8b | OSS | 0.716 [0.67, 0.77] | 80% | $0.15 (published) |
| ministral-3b | OSS | 0.713 [0.66, 0.77] | 80% | $0.10 (published) |
| gemma3-12b | OSS | 0.709 [0.65, 0.76] | 79% | $0.12 (published) |
| qwen3.5 (cloud) | OSS | 0.638 ⚠️ | 71% | artifact |
| gemma3-4b | OSS | 0.620 [0.57, 0.68] | 69% | $0.09 (published) |
| qwen3.5-397b | OSS | 0.614 ⚠️ | 69% | artifact |

> Note (2026-06-27): OSS `$/1M` are now **published serverless rates** (Together / DeepInfra /
> Mistral, blended at this token mix), replacing the earlier owned-compute estimates which were
> single-stream-throughput noise (they had ministral-14b at $23 and a 4B model above a 120B). The
> corrected frontier shows serverless OSS is cheap: gemma4:31b ($0.78) undercuts Flash-Lite at
> higher accuracy, and gpt-oss-120b ($0.49) and gemma3-27b ($0.14) are far below the hosted tier.

Hosted $/1M are measured from the gateway cost header; Granite uses published watsonx
pricing; OSS models use published serverless rates (Together / DeepInfra / Mistral, fetched
2026-06-27) blended at this token mix. (The earlier owned-compute estimate is retired: it used
single-stream *cloud* tps as a throughput proxy, which was noise, e.g. ministral-14b showed
~$23/1M at tps~3 and a 4B model priced above a 120B.) The corrected frontier puts both cheap
hosted models AND serverless OSS on the Pareto front: gemma4:31b ($0.78) undercuts Flash-Lite
at higher accuracy, and gpt-oss-120b ($0.49) / gemma3-27b ($0.14) sit well below the hosted
tier. See `provider_frontier.png`. (⚠️ Qwen rows are truncation artifacts, not true quality.)

Cheapest model meeting each quality bar (own-path price):
- 90% of Sonnet: **DeepSeek Flash, $0.26/1M**
- 95% of Sonnet: **Gemini Flash-Lite, $1.05/1M**
- 99% of Sonnet: only Sonnet

## Where the models break

The gap is concentrated in hard reasoning, not general tasks. Per-benchmark accuracy:

| Model | arc | gpqa | gsm8k | json | mbpp | mmlu | mmlupro |
|---|---|---|---|---|---|---|---|
| Sonnet | 0.98 | 0.84 | 1.00 | 1.00 | 0.70 | 0.95 | 0.86 |
| Gemini Flash-Lite | 0.98 | 0.76 | 0.98 | 1.00 | 0.63 | 0.95 | 0.80 |
| DeepSeek Flash | 0.98 | 0.68 | 1.00 | 1.00 | 0.68 | 0.93 | 0.80 |
| Haiku | 0.95 | 0.70 | 0.98 | 1.00 | 0.65 | 0.88 | 0.80 |
| Mistral Small | 0.93 | 0.38 | 0.98 | 1.00 | 0.55 | 0.85 | 0.68 |
| Granite 8B | 0.85 | 0.44 | 0.90 | 1.00 | 0.60 | 0.80 | 0.70 |

GPQA-diamond (hard science) is the separator: Sonnet 0.84 down to Mistral 0.38 and Granite
0.44. Easy slices (ARC, GSM8K) are near-saturated for everyone, and **every model passes the
JSON structured-output gate at 1.00**, so structured output does not disqualify the cheap
candidates. Implication: a cheap default only needs to escalate the hard-reasoning minority.
See `provider_heatmap.png`.

## Does routing earn its keep

No, on this pool. A bge-m3 embedding-head router choosing between the cheap default
(DeepSeek Flash) and Sonnet returned APGR 0.004 with a 95% CI of [-0.41, 0.39] - statistically
indistinguishable from just defaulting cheap. This matches the earlier Claude-pool finding:
when the cheap model is already ~95% as good, there is almost no quality gap to recover, so a
learned router adds cost and maintenance for no measurable benefit. Routing becomes worth
revisiting only on a workload with a wider, structured quality gap.

## Build vs buy

Owning compute trades fixed monthly cost for near-zero marginal cost, so it wins only above
a break-even volume. Against the cheap hosted frontier:

| Option | Fixed $/month | Break-even vs Gemini Flash-Lite ($1.05/1M) |
|---|---|---|
| Own GB10 (hal, amortized ~$0.22/hr) | ~$160 | ~150M tokens/month |
| Own L40S (cloud ~$1.56/hr) | ~$1,140 | ~1.1B tokens/month |
| Own H100 (cloud ~$2.50/hr) | ~$1,825 | ~1.75B tokens/month |
| watsonx managed | $1,500-$5,000 floor | n/a (floor, not per-token) |

Two findings sharpen this. First, against **DeepSeek Flash at $0.26/1M**, the break-even
volumes are roughly 4x higher still, because the hosted unit cost is so low. Second, and
more important: the self-host/watsonx-class model (Granite) is **quality-dominated** by the
cheap hosted options, so at any volume below break-even you would be paying fixed
infrastructure cost for *lower* quality than DeepSeek or Gemini Flash-Lite. Owning compute
is justified only at sustained high volume or when in-tenant data residency is a hard
requirement. Single-stream GB10 throughput (Granite-8B ~9 tok/s, Granite-30B ~11 tok/s) is
also slow; production economics would require batching on faster datacenter GPUs. See
`provider_breakeven.png`.

## OSS read (measured via Ollama Cloud)

The local GB10 was too saturated to sweep the big/mid OSS models, so they were measured on
Ollama Cloud (same prompts, same grading). Findings across all 19 OSS-cloud models:

- **The OSS quality ceiling is ~96% of Sonnet, and a whole cluster reaches it.** gemma4:31b
  (0.865), glm-5.2 (0.861), minimax-m3 (0.861), deepseek-v4-pro (0.854), gpt-oss-120b (0.844),
  and qwen3-coder-480b (0.841) all sit within a hair of Gemini Flash-Lite (0.858) and above
  Haiku (0.836). **No OSS model beats Sonnet (0.896)** in this set.
- **Cheap on serverless.** On published serverless rates the strong OSS cluster is not just
  competent but cheap: gemma4:31b $0.78 (undercuts Flash-Lite $1.05 at higher accuracy),
  gpt-oss-120b $0.49, gemma3-27b $0.14. So they sit ON the cost-quality frontier alongside the
  cheap hosted models (DeepSeek Flash $0.26, Mistral Small $0.09); the build-vs-buy question is
  residency and control, not unit cost. Self-hosting (owning GPUs) only pays at very high, steady
  volume (breakeven table above); renting OSS per-token is already cheap. gemma4:31b is the one to
  watch (also cheap on managed Vertex; see the conv-search forecast).
- **Mid/small OSS trails as expected.** devstral-24b 0.824, nemotron-ultra/kimi 0.817,
  gpt-oss-20b 0.792, gemma3-27b 0.771, nemotron-nano-30b/ministral-14b 0.76, gemma3-12b 0.71,
  ministral-3b 0.713 - useful frontier points but below the cheap hosted tier on quality.
  Gemma 4 (31B, 0.865) is a clear generational jump over Gemma 3 (27B, 0.771).
- **Two truncation artifacts:** qwen3.5 (0.638) and qwen3.5-397b (0.614) are extreme reasoners
  that exceed the 8192-token budget on hard slices; excluded from conclusions (would need a
  high-budget refetch).
- **Two Qwen cloud models are truncation artifacts** (qwen3.5 0.638, qwen3.5-397b 0.614):
  they are extreme reasoners that exceed the 8192-token budget on hard slices (qwen3.5-397b:
  50/290 cut off, JSON gate 0.40). Their true quality is much higher; excluded from
  conclusions pending a high-budget refetch.
- **Still completing:** nemotron-3-ultra/nano, minimax-m3, kimi-k2.7, qwen3-coder-480b,
  ministral-8b/14b, devstral-small-2:24b, gemma3-4b/12b/27b. Table updates as they land.

## Method notes and caveats

- **Reasoning-model budget fix (material):** DeepSeek, Qwen, and gpt-oss spend most of their
  token budget on hidden thinking before answering. At the initial budget they were
  truncated on hard slices and scored as failures - DeepSeek showed a false 0.30 on GPQA
  (33/50 cut off). Raising the hard-slice budget to 8k fixed it and lifted DeepSeek from a
  fake 0.741 to a true 0.850. This is the same truncation-artifact class seen earlier with
  Opus; any future model addition must be checked for it.
- **Non-reasoning runaway:** the 8k budget is pathological for non-reasoning models;
  Granite-30B looped toward 8192 tokens and wedged the GB10. Non-reasoning models are now
  capped at 2048.
- **Pricing** is homelab/list (measured gateway cost + published watsonx/GPU rates).
  In-tenant negotiated Bedrock/Vertex/watsonx rates will shift absolute numbers, not the
  shape of the frontier.
- **Quality** is public benchmarks only (chosen to separate the pool), not Marriott tasks;
  directional for real workloads, not a substitute for a representative in-domain eval.

## Recommendation

1. **Switch general-traffic default to a cheap hosted frontier model** - DeepSeek Flash
   ($0.26/1M, 95% of Sonnet) for maximum savings, or Gemini Flash-Lite ($1.05/1M, 96%) for
   the top cheap tier. In-tenant, use the Vertex/Bedrock equivalents. This is a 90-98% cost
   reduction with no router.
2. **Keep Sonnet for the hard-reasoning tail** via a simple rule, not a learned router (no
   measurable routing gain on this pool).
3. **Treat self-host/watsonx as high-volume or data-residency only.** It is Pareto-dominated
   on unit cost and quality below ~1B tokens/month; the pull toward watsonx for Marriott is
   in-tenant compliance, not price.
4. **Validate in-tenant prices** and, before committing, **build a representative in-domain
   eval** - the public-benchmark frontier is directional, and a Marriott-task eval could move
   the cheap-tier ranking.
