#!/usr/bin/env python3
"""Parameterized monthly-run-rate token-cost forecast for Marriott guest-facing
conversational search, and the savings curve across four levers:
  (1) prompt/context caching, (2) context+RAG pruning + history compaction,
  (3) model routing / downgrade, (4) self-host on Amazon.

Design notes (see docs/conv-search-token-forecast.tex):
- The cost driver is that the large system prompt (15k+) and RAG context are RE-SENT every
  turn, and conversation history grows, so per-conversation INPUT tokens scale ~quadratically
  with turns. That is why caching the fixed prefix is the dominant lever.
- All inputs are explicit and auditable here; calibrate against Adobe event206-208 volume and
  TIP/Dynatrace token telemetry, and the Egor/Kenny cost baseline. Numbers are a model, not
  measured spend. Quality/price for model-swap options reuse the LiteForge cross-provider
  study (results/provider_frontier.json).

    python scripts/forecast/conv_search_forecast.py
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
EVAL = os.path.join(HERE, "..", "eval")
OUT = os.path.join(HERE, "results")
FIG = os.path.join(OUT, "figures")

# ----------------------------------------------------------------------------- assumptions
# Volume (calibrate vs Adobe event206-208). 270M Bonvoy members; adoption = monthly active
# searchers / members; conversations per active searcher per month; turns per conversation.
VOLUME = {
    "members": 270_000_000,
    "adoption": 0.03,            # Sean's est; ~8.1M MAU. VALIDATE.
    "conv_per_user_mo": 3.0,
    "turns_per_conv": 4.0,
}
# Token mix per turn (calibrate vs Dynatrace token telemetry). System prompt confirmed 15k+.
TOKENS = {
    "system": 15_000,           # fixed instruction/tool block, re-sent every turn
    "rag": 4_000,               # retrieved context per turn
    "rag_static_frac": 0.6,     # share of RAG that is stable/cacheable (brand/policy vs live rates)
    "query": 40,                # user message tokens
    "output": 400,              # assistant output tokens per turn
}
# Current model: Gemini 3.1 Flash-Lite via the TIP.ai LiteLLM proxy (confirmed: Jira
# GENAI-1412 "Conversational Search should use Gemini 3.1 Flash Lite"; WEB-194505
# "Update mi-conversational-ai-agent to use Gemini models via TipAI LiteLLM proxy").
# Published list price $0.25 in / $1.50 out per 1M (blog.google / requesty, 2026). VALIDATE
# in-tenant rate. cache_read_mult = fraction of input price billed for a cached-prefix hit.
PRICE = {"in": 0.25, "out": 1.50, "cache_read_mult": 0.25}

# Recommended model: gemma4:31b on Vertex (managed, per-token, in-GCP - NOT Amazon self-host).
# Published Vertex price $0.15 in / $0.60 out per 1M (llmreference / tokencost, 2026). We
# MEASURED gemma4:31b vs Gemini 3.1 Flash-Lite head-to-head on the same 290 prompts: 0.866 vs
# 0.859, paired diff +0.007, 95% CI [-0.017, +0.031] => statistically ON PAR (non-inferior),
# not better. So it is ~42% CHEAPER at the input-heavy mix at EQUAL quality => a default swap
# that passes the non-inferiority (do-no-harm) gate for ~all traffic. (Amazon self-host of the
# same model is ~$4/1M: residency play, not cost - see model_options.)
SIMPLE = {"name": "gemma4:31b (Vertex, managed)", "in": 0.15, "out": 0.60}

# Lever assumptions, three scenarios (conservative / expected / aggressive).
# route_frac = share of turns served by the SIMPLE model (gemma4:31b on Vertex) instead of the
# incumbent Gemini 3.1 Flash-Lite. Because gemma4:31b is ~42% cheaper at MEASURED-on-par quality
# (0.866 vs 0.859, paired diff +0.007, 95% CI [-0.017,+0.031]), it passes the non-inferiority
# gate for ~all traffic, so this is a DEFAULT SWAP (high route_frac), keeping a hard tail on
# Flash-Lite/Gemini Pro for safety. (Self-
# host on Amazon stays a residency/control play, not a cost win - see model_options.)
SCEN = {
    "conservative": {"cache_cov": 0.50, "cache_hit": 0.50, "sys_trim": 0.20, "rag_trim": 0.20,
                     "hist_compact": 0.30, "route_frac": 0.00, "routed_extra": 0.00},
    "expected":     {"cache_cov": 0.75, "cache_hit": 0.80, "sys_trim": 0.40, "rag_trim": 0.35,
                     "hist_compact": 0.60, "route_frac": 0.90, "routed_extra": 0.02},
    "aggressive":   {"cache_cov": 0.90, "cache_hit": 0.95, "sys_trim": 0.60, "rag_trim": 0.50,
                     "hist_compact": 0.80, "route_frac": 1.00, "routed_extra": 0.04},
}


def per_conv_tokens(tk, turns, hist_compact=0.0, sys_trim=0.0, rag_trim=0.0):
    """Input/output tokens for one conversation. history at turn t = sum of prior
    (query+output); compaction caps that growth. Returns (input, output, cacheable_input)."""
    sys_ = tk["system"] * (1 - sys_trim)
    rag_ = tk["rag"] * (1 - rag_trim)
    q, o = tk["query"], tk["output"]
    T = int(round(turns))
    inp = 0.0
    cacheable = 0.0
    for t in range(1, T + 1):
        hist = (q + o) * (t - 1) * (1 - hist_compact)
        inp += sys_ + rag_ + hist + q
        cacheable += sys_ + rag_ * tk["rag_static_frac"]   # fixed prefix re-sent each turn
    out = o * T
    return inp, out, cacheable


def monthly_conversations(v):
    return v["members"] * v["adoption"] * v["conv_per_user_mo"]


def cost(inp, out, price, cached=0.0, hit=0.0):
    """USD for given token counts. `cached` input tokens hit at cache_read_mult on `hit` frac."""
    eff_cached = cached * hit
    full_in = inp - eff_cached
    cin = (full_in * price["in"] + eff_cached * price["in"] * price["cache_read_mult"]) / 1e6
    cout = out * price["out"] / 1e6
    return cin + cout


def baseline(v=VOLUME, tk=TOKENS, price=PRICE):
    conv = monthly_conversations(v)
    inp, out, _ = per_conv_tokens(tk, v["turns_per_conv"])
    c_per_conv = cost(inp, out, price)
    return {"conversations_mo": conv, "in_per_conv": inp, "out_per_conv": out,
            "usd_per_conv": c_per_conv, "usd_mo": c_per_conv * conv,
            "in_tokens_mo": inp * conv, "out_tokens_mo": out * conv}


def scenario_costs(s, v=VOLUME, tk=TOKENS, price=PRICE):
    """Apply a scenario's levers cumulatively; return $/mo after each stage."""
    conv = monthly_conversations(v)
    base_in, base_out, base_cache = per_conv_tokens(tk, v["turns_per_conv"])
    base = cost(base_in, base_out, price) * conv

    # (1) caching only (no token change; cacheable prefix hit at discount)
    cov, hit = s["cache_cov"], s["cache_hit"]
    cache_in = cost(base_in, base_out, price, cached=base_cache * cov, hit=hit) * conv

    # (2) + pruning (trim system/rag, compact history) on top of caching
    p_in, p_out, p_cache = per_conv_tokens(tk, v["turns_per_conv"], hist_compact=s["hist_compact"],
                                           sys_trim=s["sys_trim"], rag_trim=s["rag_trim"])
    prune_in = cost(p_in, p_out, price, cached=p_cache * cov, hit=hit) * conv

    # (3) + route the easy share down to the SIMPLE model (Gemma 3 on Vertex, managed).
    # Cost is linear in tokens, so blend Flash-Lite and Gemma by route_frac on the
    # post-prune/post-cache workload. Gemma also caches (same discount assumed).
    rf = s["route_frac"]
    gemma = {"in": SIMPLE["in"], "out": SIMPLE["out"], "cache_read_mult": price["cache_read_mult"]}
    prune_gemma = cost(p_in, p_out, gemma, cached=p_cache * cov, hit=hit) * conv
    model_in = (1 - rf) * prune_in + rf * prune_gemma

    # (4) + learned-router delta over the best static default (small; selection dominates)
    routed = model_in * (1 - s["routed_extra"])
    return {"baseline": base, "after_cache": cache_in, "after_prune": prune_in,
            "after_model": model_in, "after_routed": routed}


def _blended(in_p, out_p, in_share):
    return in_p * in_share + out_p * (1 - in_share)


def model_options(in_share):
    """Compare model choices for conversational search at its token mix (in_share = input
    fraction). Managed per-token options (Flash-Lite, Gemma-on-Vertex) vs Amazon self-host est.
    Blended $/1M lets us compare apples-to-apples on the chart."""
    opts = [
        {"model": "Gemini 3.1 Flash-Lite (current, Vertex)", "kind": "managed",
         "in": 0.25, "out": 1.50, "note": "incumbent; acc 0.858"},
        {"model": "gemma4:31b (Vertex, managed)", "kind": "managed",
         "in": 0.15, "out": 0.60, "note": "RECOMMENDED: ~42% cheaper, acc 0.866 on-par w/ Flash-Lite"},
        {"model": "gemma4:26b-MoE (Vertex, managed)", "kind": "managed",
         "in": 0.13, "out": 0.50, "note": "~97% of 31B perf; even cheaper (est out)"},
    ]
    for o in opts:
        o["blended_per_1m"] = round(_blended(o["in"], o["out"], in_share), 4)
    # Amazon self-host estimate from the LiteForge owned-compute frontier (residency play).
    fp = os.path.join(EVAL, "results", "provider_frontier.json")
    try:
        d = json.load(open(fp))
        for m in d["models"]:
            if m["model"] in ("gemma3-27b-cloud", "gemma-31b-cloud") and m.get("usd_per_1m"):
                opts.append({"model": f"{m['model']} (Amazon self-host est)", "kind": "selfhost",
                             "blended_per_1m": round(m["usd_per_1m"], 4),
                             "note": f"owned-compute est; acc {m['acc']:.3f}"})
                break
    except Exception:
        pass
    return opts


def cache_sweep(v=VOLUME, tk=TOKENS, price=PRICE, cov=0.75):
    base = baseline(v, tk, price)["usd_mo"]
    conv = monthly_conversations(v)
    bi, bo, bc = per_conv_tokens(tk, v["turns_per_conv"])
    rows = []
    for hit in [0.5, 0.6, 0.7, 0.8, 0.9, 0.95]:
        c = cost(bi, bo, price, cached=bc * cov, hit=hit) * conv
        rows.append({"hit": hit, "usd_mo": c, "saved_pct": round(100 * (1 - c / base), 1)})
    return rows


def tornado(v=VOLUME, tk=TOKENS, price=PRICE):
    # (label, which dict, key) for each input we sweep +/-30%.
    knobs = [("adoption", "v", "adoption"), ("conv_per_user_mo", "v", "conv_per_user_mo"),
             ("turns_per_conv", "v", "turns_per_conv"), ("system_tokens", "tk", "system"),
             ("rag_tokens", "tk", "rag"), ("in_price", "p", "in")]

    def calc(which, key, mult):
        vv, tt, pp = dict(v), dict(tk), dict(price)
        tgt = {"v": vv, "tk": tt, "p": pp}[which]
        tgt[key] = tgt[key] * mult
        return baseline(vv, tt, pp)["usd_mo"]

    out = []
    for label, which, key in knobs:
        lo, hi = calc(which, key, 0.7), calc(which, key, 1.3)
        out.append({"input": label, "low_-30pct": lo, "high_+30pct": hi, "swing": hi - lo})
    out.sort(key=lambda r: -abs(r["swing"]))
    return out


def fmt_usd(x):
    for unit, div in [("B", 1e9), ("M", 1e6), ("K", 1e3)]:
        if abs(x) >= div:
            return f"${x/div:.2f}{unit}"
    return f"${x:.2f}"


def main():
    os.makedirs(OUT, exist_ok=True)
    b = baseline()
    print("=== BASELINE (monthly run-rate; ASSUMPTIONS, calibrate vs telemetry) ===")
    print(f"  conversations/mo : {b['conversations_mo']/1e6:.1f}M  "
          f"(270M x {VOLUME['adoption']:.0%} x {VOLUME['conv_per_user_mo']} conv x "
          f"{VOLUME['turns_per_conv']} turns)")
    print(f"  input tokens/mo  : {b['in_tokens_mo']/1e12:.2f}T   output/mo: {b['out_tokens_mo']/1e9:.1f}B")
    print(f"  $/conversation   : ${b['usd_per_conv']:.4f}")
    print(f"  $/MONTH          : {fmt_usd(b['usd_mo'])}  (Gemini 3.1 Flash-Lite @ ${PRICE['in']}/{PRICE['out']} per 1M)")

    print("\n=== SAVINGS CURVE (cumulative levers, % of baseline) ===")
    scen_out = {}
    for name, s in SCEN.items():
        c = scenario_costs(s)
        sv = lambda stage: round(100 * (1 - c[stage] / c["baseline"]), 1)
        scen_out[name] = {k: c[k] for k in c}
        scen_out[name]["saved_pct"] = {"cache": sv("after_cache"), "+prune": sv("after_prune"),
                                        "+model": sv("after_model"), "+routed": sv("after_routed")}
        print(f"  {name:12s}  cache {sv('after_cache'):4.0f}% | +prune {sv('after_prune'):4.0f}% | "
              f"+model {sv('after_model'):4.0f}% | +routed {sv('after_routed'):4.0f}%  "
              f"-> {fmt_usd(c['after_routed'])}/mo")

    print("\n=== CACHE HIT-RATE SWEEP (coverage 75%) ===")
    for r in cache_sweep():
        print(f"  hit {r['hit']:.0%} -> {fmt_usd(r['usd_mo'])}/mo  (saved {r['saved_pct']}%)")

    in_share = b["in_tokens_mo"] / (b["in_tokens_mo"] + b["out_tokens_mo"])
    print(f"\n=== MODEL OPTIONS (blended $/1M at conv-search mix, input share {in_share:.0%}) ===")
    mo = model_options(in_share)
    for o in mo:
        print(f"  [{o['kind']:8s}] {o['model']:44s} ${o['blended_per_1m']:.3f}/1M  {o.get('note','')}")

    print("\n=== TORNADO (baseline $/mo sensitivity, +/-30%) ===")
    for r in tornado():
        print(f"  {r['input']:16s} swing {fmt_usd(r['swing'])}")

    res = {"assumptions": {"volume": VOLUME, "tokens": TOKENS, "price": PRICE,
                           "simple_model": SIMPLE, "scenarios": SCEN},
           "baseline": b, "scenarios": scen_out, "cache_sweep": cache_sweep(),
           "model_options": mo, "in_share": in_share, "tornado": tornado()}
    json.dump(res, open(os.path.join(OUT, "forecast.json"), "w"), indent=2, default=float)
    print(f"\nwrote {os.path.join(OUT, 'forecast.json')}")


if __name__ == "__main__":
    main()
