#!/usr/bin/env python3
"""Cost-quality evaluation on the real Claude pool (haiku/sonnet/opus).

Headline metric is the binary-frame APGR (weak=haiku, strong=opus), comparable to the
RouterBench numbers, with 95% bootstrap CIs. Also reports the 3-tier operating point
(the actual N-way product behavior) and all competitive baselines.

    python scripts/eval/claude_pool_eval.py
"""
import json
import os
import sys

import numpy as np
import pandas as pd
from sklearn.linear_model import LogisticRegression

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "scripts"))
from panel_features import extract_features, norm_struct  # noqa: E402
from baselines import keyword_scores, length_scores  # noqa: E402
from stats import bootstrap_ci, fmt_ci  # noqa: E402
from routerbench_eval import apgr, cost_saved_at_quality, curve_from_scores  # noqa: E402
from router_runner import MfRunner  # noqa: E402 (reuse its batched bge-m3 embedder)

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")
RESULTS = os.path.join(HERE, "results")
# Strong = the QUALITY leader (sonnet), not the most expensive (opus is Pareto-dominated
# on this workload: lower quality and ~9x the cost).
WEAK, STRONG = "haiku", "sonnet"
POOL = os.environ.get("CLAUDE_POOL", os.path.join(DATA, "claude_pool_auto.parquet"))


def embed_prompts(prompts):
    cache = os.path.join(DATA, "emb_cache", "claude_pool.npy")
    if os.path.exists(cache):
        e = np.load(cache)
        if len(e) == len(prompts):
            return e
    runner = MfRunner()
    vecs = runner.hardness_batch  # not used; use embed_batch
    out = []
    for i in range(0, len(prompts), 32):
        out.extend(runner.embed_batch(prompts[i:i + 32]))
        if (i + 32) % 320 == 0:
            print(f"  embedded {min(i+32,len(prompts))}/{len(prompts)}")
    arr = np.array(out, dtype=np.float32)
    os.makedirs(os.path.dirname(cache), exist_ok=True)
    np.save(cache, arr)
    return arr


def apgr_metric(scores, wq, wc, sq, sc):
    """APGR over a fixed evaluation, as a function for bootstrapping over indices."""
    oc, oq = curve_from_scores(sq - wq + 1e-6 * np.random.RandomState(0).rand(len(wq)), wq, wc, sq, sc)
    c, q = curve_from_scores(scores, wq, wc, sq, sc)
    return apgr(c, q, oc, oq, wq.mean(), sq.mean(), wc.mean(), sc.mean())


def boot_apgr(scores, wq, wc, sq, sc):
    n = len(scores)
    fn = lambda idx: apgr_metric(scores[idx], wq[idx], wc[idx], sq[idx], sc[idx])
    return bootstrap_ci(fn, n, b=1000)


def main():
    df = pd.read_parquet(POOL).reset_index(drop=True)
    print(f"claude_pool: {len(df)} prompts  "
          + str(df["eval_name"].value_counts().to_dict()))
    for t in ("haiku", "sonnet", "opus"):
        print(f"  {t}: acc={df[t].mean():.3f} cost=${df[f'{t}|total_cost'].mean():.5f}")

    prompts = df["prompt"].astype(str).str.slice(0, 4000).tolist()
    E = embed_prompts(prompts)
    S = np.array([norm_struct(extract_features(p)) for p in prompts], dtype=np.float32)
    X = np.hstack([E, S])

    wq = df[WEAK].astype(float).to_numpy(); sq = df[STRONG].astype(float).to_numpy()
    wc = df[f"{WEAK}|total_cost"].to_numpy(); sc = df[f"{STRONG}|total_cost"].to_numpy()

    # Fixed split for the head; multi-seed for training variance.
    rng = np.random.RandomState(13)
    idx = rng.permutation(len(df))
    ntr = int(len(df) * 0.7)
    tr, te = idx[:ntr], idx[ntr:]

    head_apgrs = []
    for seed in range(5):
        clf = LogisticRegression(max_iter=3000, class_weight="balanced", C=1.0, random_state=seed)
        clf.fit(X[tr], (wq[tr] >= 0.5).astype(int))
        s = 1.0 - clf.predict_proba(X[te])[:, list(clf.classes_).index(1)]
        head_apgrs.append(apgr_metric(s, wq[te], wc[te], sq[te], sc[te]))
    # final head on seed 0 for the bootstrap + scores
    clf = LogisticRegression(max_iter=3000, class_weight="balanced", C=1.0, random_state=0).fit(
        X[tr], (wq[tr] >= 0.5).astype(int))
    head_scores = 1.0 - clf.predict_proba(X[te])[:, list(clf.classes_).index(1)]

    routers = {
        "embedding head (bge-m3)": head_scores,
        "length heuristic": length_scores([prompts[i] for i in te]),
        "keyword heuristic": keyword_scores([prompts[i] for i in te]),
        "random": np.random.RandomState(7).rand(len(te)),
    }
    # semantic-router-style utterance baseline (also represents LiteLLM auto-router,
    # which is built on semantic-router): cosine to per-tier utterance centroids.
    try:
        routers["semantic (utterance)"] = semantic_scores(E[tr], wq[tr], sq[tr], E[te])
    except Exception as e:
        print("semantic baseline skipped:", e)

    wq_t, wc_t, sq_t, sc_t = wq[te], wc[te], sq[te], sc[te]
    out = {"n_test": len(te), "weak": WEAK, "strong": STRONG,
           "weak_acc": round(float(wq_t.mean()), 4), "strong_acc": round(float(sq_t.mean()), 4),
           "weak_cost": round(float(wc_t.mean()), 6), "strong_cost": round(float(sc_t.mean()), 6),
           "head_multiseed_apgr": [round(a, 4) for a in head_apgrs], "routers": {}}
    print(f"\n=== Claude-pool cost-quality (weak={WEAK}, strong={STRONG}); APGR 0=random,1=oracle ===")
    print(f"head APGR across 5 seeds: mean {np.mean(head_apgrs):+.3f} (range {min(head_apgrs):+.3f}..{max(head_apgrs):+.3f})")
    for name, sc_arr in routers.items():
        m, lo, hi = boot_apgr(sc_arr, wq_t, wc_t, sq_t, sc_t)
        c, q = curve_from_scores(sc_arr, wq_t, wc_t, sq_t, sc_t)
        saved = cost_saved_at_quality(c, q, 0.95 * sq_t.mean(), sc_t.mean())
        out["routers"][name] = {"apgr": round(m, 4), "apgr_lo": round(lo, 4), "apgr_hi": round(hi, 4),
                                "cost_saved_at_95pct": saved}
        print(f"  {name:26s} APGR {fmt_ci(m,lo,hi)}  saved@95% {saved}%")

    json.dump(out, open(os.path.join(RESULTS, "claude_pool.json"), "w"), indent=2)
    print("wrote results/claude_pool.json")


def semantic_scores(Etr, wq_tr, sq_tr, Ete):
    """Utterance-similarity baseline: build 'easy' and 'hard' centroids from train
    examples (easy = weak got it right; hard = only strong), score by cosine margin."""
    def norm(M):
        n = np.linalg.norm(M, axis=1, keepdims=True)
        return M / np.clip(n, 1e-9, None)
    easy = Etr[wq_tr >= 0.5].mean(axis=0)
    hard = Etr[(sq_tr >= 0.5) & (wq_tr < 0.5)].mean(axis=0)
    En = norm(Ete); e = easy / max(np.linalg.norm(easy), 1e-9); h = hard / max(np.linalg.norm(hard), 1e-9)
    sim_hard = En @ h; sim_easy = En @ e
    s = (sim_hard - sim_easy)
    return (s - s.min()) / max(s.max() - s.min(), 1e-9)


if __name__ == "__main__":
    main()
