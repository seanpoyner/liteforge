#!/usr/bin/env python3
"""Retrain a lightweight embedding-head router on RouterBench (the right way to
keep a router tiny: a linear head over strong pretrained bge-m3 embeddings, rather
than fine-tuning a small encoder from scratch).

Predicts whether the WEAK model is correct; route-to-strong = P(weak wrong). Trains
sklearn LogisticRegression over bge-m3 embeddings, evaluated on the same held-out
test subsample (seed 13) as the other routers. Caches embeddings to disk.

Writes scripts/eval/results/routerbench_retrained_emb.json.
"""
import json
import os

import numpy as np
import pandas as pd
from sklearn.linear_model import LogisticRegression

from router_runner import MfRunner
from routerbench_eval import (PKL, STRONG, WEAK, apgr, cost_saved_at_quality,
                              curve_from_scores, subsample)

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = os.path.join(HERE, "results")
CACHE = os.path.join(HERE, "data", "emb_cache")


def embed_cached(runner, ids, texts, tag):
    os.makedirs(CACHE, exist_ok=True)
    path = os.path.join(CACHE, f"{tag}.npy")
    if os.path.exists(path):
        return np.load(path)
    vecs = runner.embed_batch  # batched
    out = []
    for i in range(0, len(texts), 32):
        out.extend(vecs(texts[i:i + 32]))
        if (i + 32) % 320 == 0:
            print(f"  embedded {min(i+32,len(texts))}/{len(texts)} [{tag}]")
    arr = np.array(out, dtype=np.float32)
    np.save(path, arr)
    return arr


def main():
    os.makedirs(RESULTS, exist_ok=True)
    df = pd.read_pickle(PKL)
    test = subsample(df, 2000, seed=13).reset_index(drop=True)
    test_ids = set(test["sample_id"])
    train_pool = df[~df["sample_id"].isin(test_ids)]
    train = train_pool.sample(min(len(train_pool), 6000), random_state=13).reset_index(drop=True)
    print(f"train={len(train)} test={len(test)} (embedding via bge-m3)")

    runner = MfRunner()  # provides embed_batch over LiteLLM bge-m3
    Xtr = embed_cached(runner, train["sample_id"].tolist(),
                       train["prompt"].astype(str).str.slice(0, 4000).tolist(), "train6k")
    Xte = embed_cached(runner, test["sample_id"].tolist(),
                       test["prompt"].astype(str).str.slice(0, 4000).tolist(), "test2k")
    ytr = (train[WEAK].astype(float) >= 0.5).astype(int).to_numpy()  # 1 = weak correct

    clf = LogisticRegression(max_iter=2000, class_weight="balanced", C=1.0)
    clf.fit(Xtr, ytr)
    # route-to-strong propensity = P(weak wrong) = P(class 0)
    p_weak_correct = clf.predict_proba(Xte)[:, list(clf.classes_).index(1)]
    scores = 1.0 - p_weak_correct

    weak_q = test[WEAK].astype(float).to_numpy(); strong_q = test[STRONG].astype(float).to_numpy()
    weak_c = test[f"{WEAK}|total_cost"].to_numpy(); strong_c = test[f"{STRONG}|total_cost"].to_numpy()
    oracle_scores = strong_q - weak_q + 1e-6 * np.random.RandomState(0).rand(len(test))
    oc, oq = curve_from_scores(oracle_scores, weak_q, weak_c, strong_q, strong_c)
    c, q = curve_from_scores(scores, weak_q, weak_c, strong_q, strong_c)
    target = 0.95 * strong_q.mean()
    res = {
        "n": len(test), "tag": "retrained_emb",
        "router-emb": {
            "APGR": round(float(apgr(c, q, oc, oq, weak_q.mean(), strong_q.mean(),
                                     weak_c.mean(), strong_c.mean())), 4),
            "cost_saved_at_95pct_strong": cost_saved_at_quality(c, q, target, strong_c.mean()),
        },
        "curves": {"router-emb": {"cost": c.tolist(), "quality": q.tolist()},
                   "oracle": {"cost": oc.tolist(), "quality": oq.tolist()}},
    }
    json.dump(res, open(os.path.join(RESULTS, "routerbench_retrained_emb.json"), "w"))
    print(f"\n=== RouterBench retrained embedding-head (held-out test) ===")
    print(f"  router-emb APGR={res['router-emb']['APGR']:.3f}  cost_saved@95%strong={res['router-emb']['cost_saved_at_95pct_strong']}%")


if __name__ == "__main__":
    main()
