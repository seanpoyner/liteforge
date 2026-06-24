#!/usr/bin/env python3
"""RouterBench cost-quality evaluation (binary strong/weak frame, RouteLLM-style).

For a fixed test subsample, each router emits a route-to-strong propensity score.
Sweeping the routed fraction traces a cost-quality curve between all-weak and
all-strong. We compare panel / router-bert / MF against random, the single-model
points, and the oracle frontier, reporting APGR (area between the router curve and
the random line, normalized by oracle) and cost saved at 95% of strong quality.

WEAK = mixtral-8x7b-chat, STRONG = gpt-4-1106-preview (best cheap vs best strong).

Writes scripts/eval/results/routerbench.json (+ figure via plots.py).
"""
import argparse
import json
import os

import numpy as np
import pandas as pd

from router_runner import BertRunner, MfRunner, PanelRunner

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = os.path.join(HERE, "results")
PKL = os.path.join(HERE, "data", "routerbench", "routerbench_0shot.pkl")
WEAK = "mistralai/mixtral-8x7b-chat"
STRONG = "gpt-4-1106-preview"


def subsample(df, n, seed=13):
    # Stratified by eval_name for representativeness.
    return (df.groupby("eval_name", group_keys=False)
              .apply(lambda g: g.sample(min(len(g), max(1, round(n * len(g) / len(df)))),
                                        random_state=seed))
              .reset_index(drop=True))


def curve_from_scores(scores, weak_q, weak_c, strong_q, strong_c):
    """Route the top-k highest-score prompts to STRONG; return (cost, quality) per k."""
    order = np.argsort(-scores)  # high score -> strong first
    n = len(scores)
    costs, quals = [], []
    to_strong = np.zeros(n, dtype=bool)
    # k = 0..n
    costs.append(weak_c.mean()); quals.append(weak_q.mean())
    for idx in order:
        to_strong[idx] = True
        c = np.where(to_strong, strong_c, weak_c).mean()
        q = np.where(to_strong, strong_q, weak_q).mean()
        costs.append(c); quals.append(q)
    return np.array(costs), np.array(quals)


def area(costs, quals):
    # Area under quality-vs-cost curve, normalized by cost range (trapezoid).
    order = np.argsort(costs)
    return np.trapz(quals[order], costs[order])


def apgr(router_costs, router_quals, oracle_costs, oracle_quals, weak_q, strong_q, weak_c, strong_c):
    """Average Performance Gain Recovered: area between router and random, over
    area between oracle and random (both vs the random straight line)."""
    # Random line: linear interpolation between (weak_c, weak_q) and (strong_c, strong_q).
    def rand_q(c):
        if strong_c == weak_c:
            return weak_q
        return weak_q + (strong_q - weak_q) * (c - weak_c) / (strong_c - weak_c)
    def gain(costs, quals):
        order = np.argsort(costs)
        c, q = costs[order], quals[order]
        rq = np.array([rand_q(x) for x in c])
        return np.trapz(q - rq, c)
    g_router = gain(router_costs, router_quals)
    g_oracle = gain(oracle_costs, oracle_quals)
    return g_router / g_oracle if g_oracle > 1e-12 else 0.0


def cost_saved_at_quality(costs, quals, target_q, strong_c):
    """Lowest cost achieving >= target_q; report % saved vs all-strong cost."""
    order = np.argsort(costs)
    c, q = costs[order], quals[order]
    ok = c[q >= target_q]
    if len(ok) == 0:
        return None
    return round(100.0 * (1 - ok.min() / strong_c), 1)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=2000)
    ap.add_argument("--with-mf", action="store_true", help="also score MF (needs embeddings via LiteLLM)")
    ap.add_argument("--panel-dir", default=None)
    ap.add_argument("--tag", default="zeroshot")
    args = ap.parse_args()

    os.makedirs(RESULTS, exist_ok=True)
    df = pd.read_pickle(PKL)
    sub = subsample(df, args.n).reset_index(drop=True)
    print(f"test subsample: {len(sub)} prompts")

    weak_q = sub[WEAK].astype(float).to_numpy()
    strong_q = sub[STRONG].astype(float).to_numpy()
    weak_c = sub[f"{WEAK}|total_cost"].to_numpy()
    strong_c = sub[f"{STRONG}|total_cost"].to_numpy()
    prompts = sub["prompt"].astype(str).str.slice(0, 4000).tolist()

    # Oracle frontier: route prompts by quality gain (strong - weak) descending.
    oracle_scores = strong_q - weak_q + 1e-6 * np.random.RandomState(0).rand(len(sub))
    oc, oq = curve_from_scores(oracle_scores, weak_q, weak_c, strong_q, strong_c)

    routers = {}
    # Panel: route-to-strong propensity = difficulty expert P(hard).
    panel = PanelRunner(args.panel_dir)
    panel_scores = []
    diff_cls = panel.panel.classes["difficulty"]
    hard_idx = diff_cls.index("hard")
    for i, t in enumerate(prompts):
        # route-to-strong propensity = difficulty expert P(hard)
        pr = panel.panel._signal_probs("difficulty", t)
        panel_scores.append(pr[hard_idx])
        if (i + 1) % 500 == 0:
            print(f"  panel {i+1}/{len(prompts)}")
    routers["panel"] = np.array(panel_scores)

    bert = BertRunner()
    bert_scores = []
    for i, t in enumerate(prompts):
        p = bert.probs(t)
        bert_scores.append(p.get("hard", 0.0))
    routers["router-bert"] = np.array(bert_scores)

    if args.with_mf:
        mf = MfRunner()
        print("  embedding for MF (batched)...")
        routers["mf"] = np.array(mf.hardness_batch(prompts, batch=32))

    target = 0.95 * strong_q.mean()
    out = {
        "n": len(sub), "tag": args.tag,
        "weak_model": WEAK, "strong_model": STRONG,
        "weak_quality": round(float(weak_q.mean()), 4), "strong_quality": round(float(strong_q.mean()), 4),
        "weak_cost": round(float(weak_c.mean()), 6), "strong_cost": round(float(strong_c.mean()), 6),
        "routers": {}, "curves": {},
    }
    out["curves"]["oracle"] = {"cost": oc.tolist(), "quality": oq.tolist()}
    for name, sc in routers.items():
        c, q = curve_from_scores(sc, weak_q, weak_c, strong_q, strong_c)
        out["routers"][name] = {
            "APGR": round(float(apgr(c, q, oc, oq, weak_q.mean(), strong_q.mean(),
                                     weak_c.mean(), strong_c.mean())), 4),
            "cost_saved_at_95pct_strong": cost_saved_at_quality(c, q, target, strong_c.mean()),
        }
        out["curves"][name] = {"cost": c.tolist(), "quality": q.tolist()}

    json.dump(out, open(os.path.join(RESULTS, f"routerbench_{args.tag}.json"), "w"))
    print(f"\n=== RouterBench cost-quality ({args.tag}) ===")
    print(f"weak {WEAK}: q={out['weak_quality']} c={out['weak_cost']}")
    print(f"strong {STRONG}: q={out['strong_quality']} c={out['strong_cost']}")
    for name, m in out["routers"].items():
        print(f"  {name:12s} APGR={m['APGR']:.3f}  cost_saved@95%strong={m['cost_saved_at_95pct_strong']}%")
    print(f"wrote results/routerbench_{args.tag}.json")


if __name__ == "__main__":
    main()
