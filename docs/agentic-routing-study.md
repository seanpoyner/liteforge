# Agentic model routing: tau-bench study (cascade vs predictive routing)

Internal technical report. Companion to `docs/paper/liteforge-routing.tex` ("When Does Model
Routing Help?") and `docs/provider-cost-quality-study.md`. All numbers below are N=50 on the full
tau-bench airline test split, agent temperature 0, 3 trials/task, with a param-scaled compute-cost
proxy for the (free-via-gateway) OSS tiers. Code: `scripts/eval/agentic/`; results:
`scripts/eval/data/agentic_*.json`.

## TL;DR

1. **Predictive routing does not transfer to agentic tasks.** Both shipped routers (the
   `router-bert` difficulty classifier and the `MF`/RouteLLM-port over `bge-m3`) score at or below
   random APGR on tau-bench (MF significantly negative). Agentic difficulty is not legible from the
   opening user turn: "change my flight" reads easy, the episode is hard.
2. **Escalate-on-failure (cascade) is the right frame.** An oracle cascade (run cheap, escalate
   only when the cheaper tier actually fails) reaches 0.727 quality (nemotron pool) and 0.753
   (gemma pool), both *above* always-using-the-strongest model (0.628), because it keeps the cheap
   tier's wins on tasks where the strong model itself fails.
3. **The best cheap, realistic trigger is self-consistency, not an LLM judge.** Running the cheap
   tier a few times and escalating on disagreement recovers 84% (nemotron) to 98% (gemma) of
   all-strong quality at roughly half the cost, with no verifier.
4. **LLM-judge verification is calibration-bound and rarely worth it.** Only a strong, calibrated
   judge (glm-5.2, recall ~0.6) helps; small judges fail (too lenient or too harsh); none reach the
   oracle ceiling because silent failures (a wrong booking that *looks* right) cannot be verified
   from the transcript. The good judge is also the escalation target, so its cost win is small.
5. **The biggest lever is a strong cheap base model.** Swapping the weak tier from
   `nemotron-3-nano:30b` to `gemma4:31b`, the "weak" tier (0.581) beats the 397B "medium" tier
   (0.546) at 11x lower cost, and gemma alone delivers 93% of all-strong quality at 6% of the cost,
   largely removing the need to route at all.

## What we built

A real, sandboxed agentic harness (not multiple-choice), faithful to the official tau-bench
protocol, with four tools:

- `scripts/eval/agentic/run_agent.py` - OpenAI-compatible tool-calling loop against the LiteLLM
  gateway: `POOLS` (tier definitions), `call_chat` (with `finish_reason`/cost capture and a price
  fallback), `OLLAMA_PRICES` (the compute-cost proxy), and the `PROTO` cache-version tag.
- `scripts/eval/agentic/benches/taubench.py` - tau-bench adapter: real Python tools + an
  LLM-simulated user, graded by the env's DB-state reward. Matches the official `ToolCallingAgent`
  (one action = first `tool_call` per turn, `max_num_steps=30`), a Sonnet user-sim, a generic
  agent scaffold applied to every tier, full transcript capture, and `--start` for sharding.
- `scripts/eval/agentic/routing_eval.py` - builds the task x tier success/cost matrix and scores
  predictive routers (`router_runner.BertRunner`, `MfRunner`) with binary APGR (`stats.fast_apgr`),
  3-tier decision points, oracle/random/all-tier baselines, and a power calc.
- `scripts/eval/agentic/cascade_eval.py` - simulates cascade strategies from the cache plus a
  parallel LLM-judge verifier (`--judge-model`); reports the cost-quality table and judge
  precision/recall.
- Helpers: `show_transcript.py` (read an episode), `rebalance.py` (drain shard tails).

## The diagnosis journey (what we did)

The first runs showed a **tier inversion**: the stronger model scored worse than the weaker one.
Capturing transcripts and reading them traced this to three issues, none of them capability:

1. **`max_tokens` truncation of reasoning models (the dominant bug).** At `max_tokens=4096` a
   verbose/thinking model (opus, in the early gemini pool) spent its budget on reasoning and the
   response was cut off before it emitted a tool call, producing empty turns that looped to the
   step cap. A probe confirmed `truncated_calls` dropped 5 -> 0 when raised to 16384.
2. **A too-weak user simulator.** Using haiku as the tau user-sim, it broke character (it once
   replied "reservation confirmed!" right after a tool *error* and ended the episode) and approved
   constraint-violating bookings. We switched the user-sim to Sonnet.
3. **Harness non-parity.** We initially executed all parallel tool calls per turn and capped steps
   at 20; the official agent takes only the first tool call per turn with a cap of 30. We matched
   it exactly.

These fixes are versioned through the episode cache tag (`PROTO`, now `v5`). With the corrected
harness, the OSS `ollama` pool ranks monotonically and the inversion is gone; what remains are the
genuine, more interesting findings below.

## Results: cost-quality by pool (N=50 airline)

Quality = mean tau reward (DB-state correctness); cost = param-scaled $/task proxy; %strongQ/C are
relative to always-using-the-strong-tier.

### Pool A: nemotron-weak (a weak base; weak=0.474)
| strategy | quality (95% CI) | $/task | %strongQ | %strongC |
|---|---|---|---|---|
| all-weak (nemotron-3-nano:30b) | 0.474 [0.36, 0.59] | 0.012 | 76% | 14% |
| all-medium (qwen3.5:397b) | 0.546 [0.43, 0.67] | 0.056 | 87% | 68% |
| all-strong (glm-5.2) | 0.628 [0.51, 0.74] | 0.083 | 100% | 100% |
| random-tier | 0.561 | 0.057 | 89% | 69% |
| cascade: oracle (ceiling) | 0.727 [0.61, 0.83] | 0.088 | 116% | 106% |
| cascade: give-up signal | 0.487 | 0.056 | 78% | 67% |
| cascade: self-consistency | 0.526 | 0.041 | 84% | 49% |
| cascade: judge (glm-5.2:cloud) | 0.600 | 0.068 | 96% | 81% |
| predict: router-bert | 0.573 | 0.072 | 91% | 86% |
| predict: mf (RouteLLM-port) | 0.500 | 0.049 | 80% | 59% |

Routable (strong succeeds where weak fails) = 9/50 = 0.18.
Binary APGR: oracle 1.00, random ~0, router-bert -0.14, **mf -0.44** (both at or below random).

### Pool B: gemma-weak (a strong cheap base; weak=0.581)
| strategy | quality (95% CI) | $/task | %strongQ | %strongC |
|---|---|---|---|---|
| all-weak (gemma4:31b) | 0.581 [0.47, 0.70] | 0.005 | 93% | 6% |
| all-medium (qwen3.5:397b) | 0.546 [0.43, 0.67] | 0.056 | 87% | 68% |
| all-strong (glm-5.2) | 0.628 [0.51, 0.74] | 0.083 | 100% | 100% |
| random-tier | 0.588 | 0.055 | 94% | 66% |
| cascade: oracle (ceiling) | 0.753 [0.64, 0.85] | 0.071 | 120% | 86% |
| cascade: give-up signal | 0.567 | 0.040 | 90% | 47% |
| cascade: self-consistency | 0.613 | 0.035 | 98% | 42% |
| cascade: judge (glm-5.2:cloud) | 0.614 | 0.046 | 98% | 55% |
| predict: router-bert | 0.540 | 0.073 | 86% | worse than all-weak |

Routable = 9/50 = 0.18. Note `gemma4:31b` weak (0.581) > `qwen3.5:397b` medium (0.546): the 397B
"medium" tier is Pareto-dominated by a 31B model.

## The verifier-strength sweep

We ran the judge cascade with three verifiers to test whether transcript-verification scales with
judge size. It is calibration-bound, not size-monotonic:

| judge | size | precision | recall | judge-cascade quality |
|---|---|---|---|---|
| gemma3:27b-cloud | 27B | 0.50-0.58 | 0.02-0.07 (too lenient) | ~0.51-0.58 (≈ all-weak) |
| qwen3:14b | 14B | 0.64 | 0.44 (harsh, many false escalations) | 0.519 |
| glm-5.2:cloud | strong tier | 0.67-0.68 | 0.55-0.60 (best) | 0.600 / 0.614 |
| oracle (perfect detector) | - | 1.00 | 1.00 | 0.727 / 0.753 |

Even the strongest judge misses ~40% of failures, because the failures it misses are *silent*: the
agent says "booked!" whether the booking was right or wrong, so success and failure transcripts
look alike. Verification of agentic success is roughly as hard as the task itself.

## Why predictive routing fails here (and where it works)

The same `bge-m3` embedding head that recovers real cost on RouterBench (APGR 0.308, see the paper)
lands at or below random on tau-bench. The difference is the decision signal: RouterBench routes on
a self-contained QA prompt whose difficulty is legible; an agentic episode's difficulty emerges
only during multi-step tool use and policy adherence (for example the airline "at most one travel
certificate" rule), which the opening user message does not reveal. Routing up front is close to
blind. This sharpens, not contradicts, the paper's thesis: a router is only as valuable as a
quality gap it can *see in advance*.

## Recommendation

For agentic workloads:

1. **Default to a strong cheap base model.** `gemma4:31b` is the standout: it beats a 397B tier at
   a fraction of the cost and reaches 93% of the strongest tier's quality alone. This is the single
   largest lever and it agrees with the cross-provider study (`provider-cost-quality-study.md`,
   gemma4:31b 0.865 = 96% of Sonnet).
2. **Escalate on failure, do not predict difficulty up front.** Cascade beats predictive routing,
   and an oracle cascade beats even always-strong.
3. **Use self-consistency as the cheap escalation trigger.** Run the base a few times, escalate on
   disagreement: ~half the cost of always-strong at 84-98% of its quality, no verifier needed.
4. **Reserve an LLM-judge verifier for cases that justify a strong, calibrated judge.** A small or
   mis-calibrated judge adds cost without recall; even a strong judge leaves the silent-failure
   headroom on the table.
5. **Skip the predictive difficulty router for agentic routing** (keep it for QA-like domains).

## Methodology notes and gotchas

- Reasoning/verbose models need a generous `max_tokens` (we use 16384 for the agent, 2048 for the
  judge) or they truncate before emitting their tool call / verdict; parse the explicit final
  `VERDICT:` line and fail-open so a truncated judgment never spuriously escalates.
- Gateway model ids matter: the strong tier is `glm-5.2:cloud` (the bare `glm-5.2` 400s and, with
  fail-open handling, silently degrades a judge to "never escalate").
- Some models are unloaded/cold on the gateway and time out (`qwen3.6:27b`, `granite4.1:30b`,
  `nemotron3:33b`); responsive sub-40B judges include `gemma3:27b-cloud`, `gpt-oss:20b-cloud`,
  `qwen3:14b`.
- Per-task tau variance is large (frontier pass^1 is ~0.4-0.55); N=5 pilots can show spurious
  inversions. The full 50-task split (the whole airline test set) is what these conclusions rest
  on. retail is a quality ceiling (all tiers ~1.0) and carries no routing signal.
- Cost axis: OSS tiers are free via the gateway, so the economic axis uses a param-scaled $/Mtok
  proxy applied to measured tokens (`OLLAMA_PRICES`); only the monotonic ordering by tier matters
  for the routing shape, and the constants are a documented assumption.
