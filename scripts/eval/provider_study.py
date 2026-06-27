#!/usr/bin/env python3
"""Cross-provider cost-quality + build-vs-buy analysis over the provider pool.

Consumes build_provider_pool.py's parquet and produces:
  - per-model quality (overall + per benchmark) with bootstrap 95% CIs
  - three pricing overlays -> $/1M tokens: hosted (measured), owned-compute (from measured
    single-stream tps + batching/utilization), watsonx managed (published Granite rates)
  - the cost-quality Pareto frontier and the cheapest model meeting each quality bar
  - routing ROI: does a bge-m3 head beat the best static cheap default? (APGR, CIs)
  - build-vs-buy break-even: owned-compute vs hosted vs watsonx as a function of volume

    python scripts/eval/provider_study.py [--pool data/provider_pool.parquet]

Pricing sources (fetched 2026-06-24): watsonx Granite-4-h-small $0.06/$0.25 per 1M
(ibm.com/products/watsonx-ai/pricing); cloud GPU ~$1.56 L40S / ~$2.50 H100 per hr
(intuitionlabs.ai, spheron.network). Hosted prices are MEASURED from the gateway's
x-litellm-response-cost header; the tables below are fallbacks only.
"""
import argparse
import json
import os
import sys

import numpy as np
import pandas as pd
from sklearn.linear_model import LogisticRegression

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "scripts"))
from stats import bootstrap_ci, fmt_ci  # noqa: E402
from routerbench_eval import apgr, cost_saved_at_quality, curve_from_scores  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")
RESULTS = os.path.join(HERE, "results")

# alias -> economic path (mirrors build_provider_pool.MODELS).
PATH = {
    "gemini-flash-lite": "hosted", "gpt-nano": "hosted", "gpt-mini": "hosted",
    "deepseek-flash": "hosted", "mistral-small": "hosted",
    "granite-8b": "selfhost", "granite-30b": "selfhost", "qwen-4b": "selfhost",
    "qwen-27b": "selfhost", "gemma-e4b": "selfhost", "gpt-oss-120b": "selfhost",
    "haiku": "anchor", "sonnet": "anchor",
    # OSS measured via Ollama Cloud; economically they belong to the self-host/"buy compute"
    # path (you would self-host them), so they are priced on owned-compute estimates.
    "gpt-oss-120b-cloud": "cloud", "qwen-cloud": "cloud", "gemma-31b-cloud": "cloud",
    "gpt-oss-20b-cloud": "cloud", "deepseek-pro-cloud": "cloud", "qwen-397b-cloud": "cloud",
    "glm-cloud": "cloud", "nemotron-ultra-cloud": "cloud", "minimax-cloud": "cloud",
    "kimi-cloud": "cloud", "qwen-coder-480b-cloud": "cloud",
    "ministral-3b-cloud": "cloud", "ministral-8b-cloud": "cloud", "ministral-14b-cloud": "cloud",
    "devstral-24b-cloud": "cloud", "nemotron-nano-30b-cloud": "cloud",
    "gemma3-4b-cloud": "cloud", "gemma3-12b-cloud": "cloud", "gemma3-27b-cloud": "cloud",
}
# Published per-1M (in, out) fallbacks; hosted uses measured cost when available.
PRICE_1M = {
    "haiku": (1.0, 5.0), "sonnet": (3.0, 15.0),
    "gemini-flash-lite": (0.10, 0.40), "deepseek-flash": (0.27, 1.10),
    "mistral-small": (0.10, 0.30),
}
# watsonx managed (published Granite tiers, USD/1M in/out). granite-30b is an estimate.
WATSONX_1M = {"granite-8b": (0.06, 0.25), "granite-30b": (0.20, 0.60)}
# Published serverless-inference rates for the OSS-cloud models (USD/1M in/out), fetched
# 2026-06-27: Together AI (together.ai/pricing) for gemma4-31b, glm-5.2, minimax-m3,
# deepseek-v4-pro, nemotron-3-ultra, kimi, gpt-oss-20b/120b, qwen3.5-397b/9b; DeepInfra
# (deepinfra.com/pricing) for qwen3-coder-480b, gemma3-4b/12b/27b, nemotron-nano-30b;
# Mistral (mistral.ai/pricing) for ministral-3b/8b/14b, devstral-small-2. This replaces the
# earlier owned-compute estimate, which was throughput-noise (single-stream cloud tps).
PUBLISHED_OSS_1M = {
    "gemma-31b-cloud": (0.39, 0.97), "glm-cloud": (1.40, 4.40), "minimax-cloud": (0.30, 1.20),
    "deepseek-pro-cloud": (1.74, 3.48), "gpt-oss-120b-cloud": (0.15, 0.60),
    "qwen-coder-480b-cloud": (0.30, 1.00), "nemotron-ultra-cloud": (0.60, 3.60),
    "kimi-cloud": (1.20, 4.50), "gpt-oss-20b-cloud": (0.05, 0.20),
    "gemma3-27b-cloud": (0.08, 0.16), "ministral-14b-cloud": (0.20, 0.20),
    "nemotron-nano-30b-cloud": (0.05, 0.20), "ministral-8b-cloud": (0.15, 0.15),
    "ministral-3b-cloud": (0.10, 0.10), "gemma3-12b-cloud": (0.05, 0.15),
    "gemma3-4b-cloud": (0.05, 0.10), "devstral-24b-cloud": (0.10, 0.30),
    "qwen-cloud": (0.17, 0.25), "qwen-397b-cloud": (0.60, 3.60),
}
# Owned-compute scenarios: GPU $/hr. GB10 = hal's actual DGX Spark amortized
# (~$4000 / 3yr + power ~= $0.22/hr); datacenter cards for a "buy real compute" read.
GPU_HR = {"GB10 (hal, amortized)": 0.22, "L40S (cloud)": 1.56, "H100 (cloud)": 2.50}
# Effective throughput multiplier from batching/concurrency at production utilization.
# Single-stream measured tps is batch=1 (worst case); served systems batch many requests.
BATCH_MULT = [1, 10, 30]


def models_in(df):
    return [a for a in PATH if f"{a}|total_cost" in df.columns]


def measured_per_1m(df, a):
    """Effective USD per 1M total tokens from measured cost (hosted models only)."""
    cost = df[f"{a}|total_cost"].sum()
    toks = (df[f"{a}|pt"].sum() + df[f"{a}|ct"].sum())
    return (cost / toks * 1e6) if cost > 0 and toks > 0 else None


def blended_1m(price_in_out, df, a):
    pin, pout = price_in_out
    pt, ct = df[f"{a}|pt"].mean(), df[f"{a}|ct"].mean()
    tot = pt + ct
    return (pin * pt + pout * ct) / tot if tot else None


def owned_1m(tps, gpu_hr, mult):
    """USD/1M tokens for self-hosting: GPU $/hr over effective tokens/hr."""
    if tps <= 0:
        return None
    eff_tok_per_hr = tps * mult * 3600
    return gpu_hr / eff_tok_per_hr * 1e6


def best_price_1m(df, a):
    """The representative $/1M for a model on its own economic path (for the frontier)."""
    p = PATH[a]
    if p in ("hosted", "anchor"):
        m = measured_per_1m(df, a)
        if m is not None:
            return m, "hosted(measured)"
        if a in PRICE_1M:
            return blended_1m(PRICE_1M[a], df, a), "hosted(published)"
        return None, "n/a"
    if p == "cloud":
        # OSS-cloud models are priced at their published serverless rate (blended at this
        # token mix), not an owned-compute estimate: single-stream cloud tps was throughput
        # noise that inverted the cost ordering (e.g. a 14B model at $23/1M).
        if a in PUBLISHED_OSS_1M:
            return blended_1m(PUBLISHED_OSS_1M[a], df, a), "published"
        return None, "n/a"
    tps = df[f"{a}|tps"][df[f"{a}|tps"] > 0].mean()
    # self-host: report the most favorable realistic owned-compute (GB10 amortized,
    # batch=10) and, for granite, also watsonx managed; take the cheaper.
    owned = owned_1m(tps, GPU_HR["GB10 (hal, amortized)"], 10)
    cands = [(owned, "owned(GB10,b10)")]
    if a in WATSONX_1M:
        cands.append((blended_1m(WATSONX_1M[a], df, a), "watsonx"))
    cands = [(c, lbl) for c, lbl in cands if c is not None]
    return min(cands) if cands else (None, "n/a")


def acc_ci(df, a):
    y = df[a].to_numpy()
    m, lo, hi = bootstrap_ci(lambda idx: y[idx].mean(), len(y), b=1000)
    return m, lo, hi


def embed(df):
    from router_runner import MfRunner
    cache = os.path.join(DATA, "emb_cache", "provider_pool.npy")
    prompts = df["prompt"].astype(str).str.slice(0, 4000).tolist()
    if os.path.exists(cache):
        e = np.load(cache)
        if len(e) == len(prompts):
            return e
    r = MfRunner()
    out = []
    for i in range(0, len(prompts), 32):
        out.extend(r.embed_batch(prompts[i:i + 32]))
    arr = np.array(out, dtype=np.float32)
    os.makedirs(os.path.dirname(cache), exist_ok=True)
    np.save(cache, arr)
    return arr


def routing_roi(df, weak, strong):
    """Does a bge-m3 head beat the best static cheap default? Binary frame weak vs strong."""
    E = embed(df)
    wq = df[weak].to_numpy(float); sq = df[strong].to_numpy(float)
    wc = df[f"{weak}|total_cost"].to_numpy(float); sc = df[f"{strong}|total_cost"].to_numpy(float)
    # measured costs may be 0 for self-host weak; fall back to a tiny epsilon so the curve
    # is well-defined (the frontier uses $/1M separately).
    if sc.sum() <= 0:
        return None
    rng = np.random.RandomState(13)
    idx = rng.permutation(len(df)); ntr = int(len(df) * 0.7)
    tr, te = idx[:ntr], idx[ntr:]
    clf = LogisticRegression(max_iter=3000, class_weight="balanced").fit(
        E[tr], (wq[tr] >= 0.5).astype(int))
    head = 1.0 - clf.predict_proba(E[te])[:, list(clf.classes_).index(1)]
    wqt, sqt, wct, sct = wq[te], sq[te], wc[te], sc[te]

    def apgr_of(scores):
        oc, oq = curve_from_scores(sqt - wqt + 1e-9, wqt, wct, sqt, sct)
        c, q = curve_from_scores(scores, wqt, wct, sqt, sct)
        return apgr(c, q, oc, oq, wqt.mean(), sqt.mean(), wct.mean(), sct.mean())

    m, lo, hi = bootstrap_ci(
        lambda i: apgr_of(head[i]) if len(set(i)) > 2 else 0.0, len(te), b=500)
    return {"weak": weak, "strong": strong, "n_test": len(te),
            "weak_acc": round(float(wqt.mean()), 4), "strong_acc": round(float(sqt.mean()), 4),
            "head_apgr": round(m, 4), "head_apgr_lo": round(lo, 4), "head_apgr_hi": round(hi, 4)}


def breakeven(hosted_1m, owned_fixed_monthly, owned_marginal_1m):
    """Monthly tokens where owned-compute total cost == hosted total cost."""
    denom = (hosted_1m - owned_marginal_1m)
    if denom <= 0:
        return None
    return owned_fixed_monthly / denom * 1e6  # tokens/month


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pool", default=os.path.join(DATA, "provider_pool.parquet"))
    ap.add_argument("--out", default=os.path.join(RESULTS, "provider_frontier.json"))
    args = ap.parse_args()
    df = pd.read_parquet(args.pool).reset_index(drop=True)
    aliases = models_in(df)
    benches = sorted(df["eval_name"].unique())
    print(f"provider pool: {len(df)} prompts {df['eval_name'].value_counts().to_dict()}")
    print(f"models: {aliases}\n")

    sonnet_acc = df["sonnet"].mean() if "sonnet" in aliases else max(df[a].mean() for a in aliases)
    rows = []
    for a in aliases:
        m, lo, hi = acc_ci(df, a)
        price, src = best_price_1m(df, a)
        tps = df[f"{a}|tps"][df[f"{a}|tps"] > 0].mean()
        rows.append({"model": a, "path": PATH[a], "acc": round(m, 4),
                     "acc_lo": round(lo, 4), "acc_hi": round(hi, 4),
                     "pct_of_sonnet": round(m / sonnet_acc * 100, 1) if sonnet_acc else None,
                     "usd_per_1m": round(price, 4) if price else None, "price_src": src,
                     "tps_single": round(float(tps), 1) if tps == tps else None,
                     "per_bench": {b: round(df[df.eval_name == b][a].mean(), 3) for b in benches}})
    rows.sort(key=lambda r: (-(r["acc"])))

    print(f"{'model':18s} {'path':9s} {'acc(95% CI)':22s} {'%son':>5s} {'$/1M':>8s} {'src':16s} tps")
    for r in rows:
        print(f"{r['model']:18s} {r['path']:9s} "
              f"{fmt_ci(r['acc'], r['acc_lo'], r['acc_hi']):22s} "
              f"{str(r['pct_of_sonnet']):>5s} {str(r['usd_per_1m']):>8s} {r['price_src']:16s} "
              f"{r['tps_single']}")

    print("\nper-benchmark accuracy:")
    hdr = "model".ljust(18) + "".join(b[:8].rjust(9) for b in benches)
    print(hdr)
    for r in rows:
        print(r["model"].ljust(18) + "".join(f"{r['per_bench'][b]:9.3f}" for b in benches))

    # Cheapest model meeting each quality bar (on its own-path price).
    print("\ncheapest model at quality bar (own-path $/1M):")
    priced = [r for r in rows if r["usd_per_1m"]]
    bars = {"90% sonnet": 0.90, "95% sonnet": 0.95, "99% sonnet": 0.99}
    bar_out = {}
    for name, frac in bars.items():
        ok = [r for r in priced if r["acc"] >= frac * sonnet_acc]
        best = min(ok, key=lambda r: r["usd_per_1m"]) if ok else None
        bar_out[name] = best["model"] if best else None
        if best:
            print(f"  {name:12s} -> {best['model']:18s} ${best['usd_per_1m']:.3f}/1M "
                  f"(acc {best['acc']:.3f}, {best['path']})")
        else:
            print(f"  {name:12s} -> none in pool")

    # Routing ROI: weak = cheapest model >= 90% sonnet, strong = sonnet.
    roi = None
    if "sonnet" in aliases:
        ok = [r for r in priced if r["acc"] >= 0.90 * sonnet_acc and r["model"] != "sonnet"]
        weak = min(ok, key=lambda r: r["usd_per_1m"])["model"] if ok else \
            min(priced, key=lambda r: r["usd_per_1m"])["model"]
        roi = routing_roi(df, weak, "sonnet")
        if roi:
            print(f"\nrouting ROI (weak={weak} {roi['weak_acc']:.3f}, strong=sonnet "
                  f"{roi['strong_acc']:.3f}): head APGR "
                  f"{fmt_ci(roi['head_apgr'], roi['head_apgr_lo'], roi['head_apgr_hi'])}")
            print("  (APGR>0 => routing beats the static cheap default; CI spanning 0 => it does not)")

    # Build-vs-buy break-even: a representative cheap hosted model vs owning one GPU.
    be = {}
    cheap_hosted = next((r for r in rows if r["path"] == "hosted" and r["usd_per_1m"]), None)
    if cheap_hosted:
        h = cheap_hosted["usd_per_1m"]
        for gpu, hr in GPU_HR.items():
            fixed = hr * 730  # one GPU, 1 month
            tok = breakeven(h, fixed, owned_marginal_1m=0.0)  # marginal ~ electricity ~ 0
            be[gpu] = {"fixed_monthly_usd": round(fixed, 2),
                       "breakeven_tokens_month": round(tok) if tok else None}
        print(f"\nbuild-vs-buy break-even vs hosted '{cheap_hosted['model']}' (${h:.3f}/1M):")
        for gpu, d in be.items():
            t = d["breakeven_tokens_month"]
            print(f"  own {gpu:22s} fixed ${d['fixed_monthly_usd']:.0f}/mo -> break-even at "
                  f"{(t/1e6):.1f}M tok/mo" if t else f"  own {gpu}: n/a")
        be["watsonx_floor_usd_month"] = [1500, 5000]
        print("  watsonx managed: $1,500-$5,000/mo minimum spend floor per region")

    out = {"n_prompts": len(df), "benches": benches, "sonnet_acc": round(float(sonnet_acc), 4),
           "models": rows, "quality_bars": bar_out, "routing_roi": roi, "breakeven": be,
           "pricing_notes": {"gpu_hr": GPU_HR, "watsonx_1m": WATSONX_1M,
                             "batch_mult_modeled": BATCH_MULT,
                             "sources_fetched": "2026-06-24"}}
    os.makedirs(RESULTS, exist_ok=True)
    json.dump(out, open(args.out, "w"), indent=2)
    print(f"\nwrote {args.out}")


if __name__ == "__main__":
    main()
