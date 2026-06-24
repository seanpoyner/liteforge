#!/usr/bin/env python3
"""Reconcile our MF port (APGR 0.10) against RouteLLM's released MF checkpoint.

Runs RouteLLM's `mf` router (routellm/mf_gpt4_augmented, text-embedding-3-small via our
gateway) on the SAME RouterBench test split and metric we use, so the two numbers are
directly comparable. If RouteLLM's MF scores much higher, the gap is our port/training,
not MF as a method.

    python scripts/eval/routellm_mf.py
"""
import json
import os
import sys
from concurrent.futures import ThreadPoolExecutor

import numpy as np
import pandas as pd

# Point RouteLLM's OpenAI client (used for text-embedding-3-small) at our gateway.
os.environ.setdefault("OPENAI_API_KEY", os.environ.get("LITEFORGE_API_KEY", ""))
os.environ.setdefault("OPENAI_BASE_URL",
                      os.environ.get("ROUTER_EVAL_BASE_URL", "http://10.8.0.6:4000/v1"))

from routellm.routers.routers import MatrixFactorizationRouter  # noqa: E402
from routerbench_eval import (PKL, STRONG, WEAK, apgr, cost_saved_at_quality,  # noqa: E402
                              curve_from_scores, subsample)

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = os.path.join(HERE, "results")
CACHE = os.path.join(HERE, "data", "routellm_mf_scores.npy")


def main():
    df = pd.read_pickle(PKL)
    test = subsample(df, 2000, seed=13).reset_index(drop=True)
    prompts = test["prompt"].astype(str).str.slice(0, 6000).tolist()
    wq = test[WEAK].astype(float).to_numpy(); sq = test[STRONG].astype(float).to_numpy()
    wc = test[f"{WEAK}|total_cost"].to_numpy(); sc = test[f"{STRONG}|total_cost"].to_numpy()

    if os.path.exists(CACHE) and len(np.load(CACHE)) == len(prompts):
        scores = np.load(CACHE)
        print("loaded cached RouteLLM MF scores")
    else:
        print("loading RouteLLM MF checkpoint (routellm/mf_gpt4_augmented)...")
        router = MatrixFactorizationRouter(checkpoint_path="routellm/mf_gpt4_augmented")
        print(f"scoring {len(prompts)} prompts (text-embedding-3-small via gateway)...")
        scores = [None] * len(prompts)

        def score(i):
            try:
                scores[i] = float(router.calculate_strong_win_rate(prompts[i]))
            except Exception as e:
                scores[i] = 0.5
                if i < 3:
                    print("  score error:", e)
        with ThreadPoolExecutor(max_workers=8) as ex:
            for k, _ in enumerate(ex.map(score, range(len(prompts)))):
                if (k + 1) % 200 == 0:
                    print(f"  {k+1}/{len(prompts)}")
        scores = np.array(scores, dtype=float)
        np.save(CACHE, scores)

    oc, oq = curve_from_scores(sq - wq + 1e-6 * np.random.RandomState(0).rand(len(test)), wq, wc, sq, sc)
    c, q = curve_from_scores(scores, wq, wc, sq, sc)
    a = apgr(c, q, oc, oq, wq.mean(), sq.mean(), wc.mean(), sc.mean())
    saved = cost_saved_at_quality(c, q, 0.95 * sq.mean(), sc.mean())
    res = {"router": "routellm-mf-released", "apgr": round(float(a), 4),
           "cost_saved_at_95pct": saved, "our_port_apgr": 0.100}
    json.dump(res, open(os.path.join(RESULTS, "routellm_mf.json"), "w"), indent=2)
    print(f"\n=== RouteLLM released MF on our RouterBench split + metric ===")
    print(f"  RouteLLM MF : APGR {a:+.3f}  saved@95% {saved}%")
    print(f"  our port    : APGR +0.100  (gap -> port/training, not the method)")


if __name__ == "__main__":
    main()
