#!/usr/bin/env python3
"""RouterBench baselines with bootstrap CIs: embedding head vs trivial heuristics vs
semantic-utterance routing vs random, all on the seed-13 test split under our metric.

Reuses the cached bge-m3 embeddings from retrain_emb.py (train6k/test2k.npy).

    python scripts/eval/routerbench_baselines.py
"""
import json
import os
import sys

import numpy as np
import pandas as pd
from sklearn.linear_model import LogisticRegression

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "scripts"))
from baselines import keyword_scores, length_scores  # noqa: E402
from stats import bootstrap_ci, fast_apgr, fmt_ci  # noqa: E402
from routerbench_eval import (PKL, STRONG, WEAK, cost_saved_at_quality,  # noqa: E402
                              curve_from_scores, subsample)

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")
RESULTS = os.path.join(HERE, "results")


def main():
    df = pd.read_pickle(PKL)
    test = subsample(df, 2000, seed=13).reset_index(drop=True)
    test_ids = set(test["sample_id"])
    train = df[~df["sample_id"].isin(test_ids)].sample(6000, random_state=13).reset_index(drop=True)
    Etr = np.load(os.path.join(DATA, "emb_cache", "train6k.npy"))
    Ete = np.load(os.path.join(DATA, "emb_cache", "test2k.npy"))
    assert len(Ete) == len(test) and len(Etr) == len(train), "cache mismatch; run retrain_emb.py"

    wq = test[WEAK].astype(float).to_numpy(); sq = test[STRONG].astype(float).to_numpy()
    wc = test[f"{WEAK}|total_cost"].to_numpy(); sc = test[f"{STRONG}|total_cost"].to_numpy()
    prompts = test["prompt"].astype(str).tolist()

    # embedding head (logistic, predict weak-correct)
    clf = LogisticRegression(max_iter=3000, class_weight="balanced").fit(
        Etr, (train[WEAK].astype(float) >= 0.5).astype(int))
    head = 1.0 - clf.predict_proba(Ete)[:, list(clf.classes_).index(1)]

    # semantic utterance baseline (represents semantic-router / LiteLLM auto-router)
    def norm(M):
        return M / np.clip(np.linalg.norm(M, axis=1, keepdims=True), 1e-9, None)
    wtr = (train[WEAK].astype(float) >= 0.5).to_numpy(); str_ = (train[STRONG].astype(float) >= 0.5).to_numpy()
    easy = Etr[wtr].mean(0); hard = Etr[str_ & ~wtr].mean(0)
    En = norm(Ete)
    sem = En @ (hard / np.linalg.norm(hard)) - En @ (easy / np.linalg.norm(easy))
    sem = (sem - sem.min()) / max(sem.max() - sem.min(), 1e-9)

    routers = {
        "embedding head (bge-m3)": head,
        "length heuristic": length_scores(prompts),
        "keyword heuristic": keyword_scores(prompts),
        "semantic (utterance)": sem,
        "random": np.random.RandomState(7).rand(len(test)),
    }
    out = {"n_test": len(test), "routers": {}}
    print(f"\n=== RouterBench baselines (seed-13 test, n={len(test)}); APGR 0=random,1=oracle ===")
    for name, s in routers.items():
        s_ = s
        m, lo, hi = bootstrap_ci(lambda idx, s_=s_: fast_apgr(s_[idx], wq[idx], wc[idx], sq[idx], sc[idx]),
                                 len(test), b=1000)
        c, q = curve_from_scores(s, wq, wc, sq, sc)
        saved = cost_saved_at_quality(c, q, 0.95 * sq.mean(), sc.mean())
        out["routers"][name] = {"apgr": round(m, 4), "apgr_lo": round(lo, 4), "apgr_hi": round(hi, 4),
                                "cost_saved_at_95pct": saved}
        print(f"  {name:26s} APGR {fmt_ci(m,lo,hi)}  saved@95% {saved}%")
    json.dump(out, open(os.path.join(RESULTS, "routerbench_baselines.json"), "w"), indent=2)
    print("wrote results/routerbench_baselines.json")


if __name__ == "__main__":
    main()
