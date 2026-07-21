#!/usr/bin/env python3
"""Train + optimize the embedding-head router and export ~/.forge/router-head.json.

Reuses the cached bge-m3 embeddings from retrain_emb.py (data/emb_cache/{train6k,test2k}.npy,
which correspond to the same deterministic row selection) and the cost-quality metrics
from routerbench_eval.py.

Quality head: searches {logistic, MLP} x {binary weak-suffices, 3-class oracle-tier}
x {embedding only, embedding+structural}, scores each by held-out RouterBench APGR, and
keeps the best. Task head: logistic/MLP over embeddings -> {qa, code, math} from
provenance labels. Exports a generic dense-layer head spec the Rust selector can run.
"""
import json
import os
import sys

import numpy as np
import pandas as pd
from sklearn.linear_model import LogisticRegression
from sklearn.neural_network import MLPClassifier

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "scripts"))
from panel_features import (CTX_TOKEN_SCALE, N_FILES_SCALE, STRUCT_FEATURES,  # noqa: E402
                            extract_features, norm_struct)
from fetch_data import MID, STRONG, WEAK, task_label  # noqa: E402
from routerbench_eval import (apgr, cost_saved_at_quality, curve_from_scores,  # noqa: E402
                              subsample, PKL)

HERE = os.path.dirname(os.path.abspath(__file__))
CACHE = os.path.join(HERE, "data", "emb_cache")
OUT = os.path.expanduser("~/.forge/router-head.json")


def struct_matrix(prompts):
    return np.array([norm_struct(extract_features(p)) for p in prompts], dtype=np.float32)


def layers_from_logreg(clf):
    # One dense layer: logits = W x + b ; W is [n_classes, n_features].
    W = np.atleast_2d(clf.coef_)
    b = np.atleast_1d(clf.intercept_)
    if W.shape[0] == 1:  # binary LR -> expand to 2-class logits [0, wx+b]
        W = np.vstack([np.zeros_like(W[0]), W[0]])
        b = np.array([0.0, b[0]])
    return [{"W": W.tolist(), "b": b.tolist(), "activation": "none"}], list(clf.classes_)


def layers_from_mlp(clf):
    layers = []
    n = len(clf.coefs_)
    for i, (coef, inter) in enumerate(zip(clf.coefs_, clf.intercepts_)):
        # sklearn stores coef as [in, out]; our forward uses [out, in].
        layers.append({"W": coef.T.tolist(), "b": list(inter),
                       "activation": "relu" if i < n - 1 else "none"})
    return layers, list(clf.classes_)


def make_clf(kind):
    if kind == "logistic":
        return LogisticRegression(max_iter=3000, class_weight="balanced", C=1.0)
    return MLPClassifier(hidden_layer_sizes=(64,), max_iter=300, early_stopping=False,
                         random_state=13)


def route_to_strong_scores(proba, classes):
    """Scalar route-to-strong propensity from class probabilities."""
    cls = list(classes)
    if set(cls) <= {0, 1}:                      # binary: 1 = weak correct
        return 1.0 - proba[:, cls.index(1)]
    # 3-class tiers
    idx = {c: cls.index(c) for c in cls}
    s = np.zeros(proba.shape[0])
    if "strong" in idx: s += proba[:, idx["strong"]]
    if "mid" in idx: s += 0.5 * proba[:, idx["mid"]]
    return s


def main():
    df = pd.read_pickle(PKL)
    test = subsample(df, 2000, seed=13).reset_index(drop=True)
    test_ids = set(test["sample_id"])
    train = df[~df["sample_id"].isin(test_ids)].sample(min(len(df) - len(test_ids), 6000),
                                                       random_state=13).reset_index(drop=True)
    Etr = np.load(os.path.join(CACHE, "train6k.npy"))
    Ete = np.load(os.path.join(CACHE, "test2k.npy"))
    assert len(Etr) == len(train) and len(Ete) == len(test), "cache/row mismatch; rerun retrain_emb.py"
    print(f"train={len(train)} test={len(test)} emb_dim={Etr.shape[1]}")

    Str = struct_matrix(train["prompt"].astype(str).str.slice(0, 4000).tolist())
    Ste = struct_matrix(test["prompt"].astype(str).str.slice(0, 4000).tolist())

    # Targets.
    wc = (train[WEAK].astype(float) >= 0.5).astype(int).to_numpy()          # binary
    def tier_label(r):
        if float(r[WEAK]) >= 0.5: return "weak"
        if float(r[MID]) >= 0.5: return "mid"
        return "strong"
    ttr = train.apply(tier_label, axis=1).to_numpy()                        # 3-class

    weak_q = test[WEAK].astype(float).to_numpy(); strong_q = test[STRONG].astype(float).to_numpy()
    weak_c = test[f"{WEAK}|total_cost"].to_numpy(); strong_c = test[f"{STRONG}|total_cost"].to_numpy()
    oracle_scores = strong_q - weak_q + 1e-6 * np.random.RandomState(0).rand(len(test))
    oc, oq = curve_from_scores(oracle_scores, weak_q, weak_c, strong_q, strong_c)
    target_q = 0.95 * strong_q.mean()

    best = None
    print("\nvariant search (held-out APGR):")
    for kind in ("logistic", "mlp"):
        for tgt in ("binary", "tier3"):
            for use_struct in (False, True):
                Xtr = np.hstack([Etr, Str]) if use_struct else Etr
                Xte = np.hstack([Ete, Ste]) if use_struct else Ete
                y = wc if tgt == "binary" else ttr
                clf = make_clf(kind).fit(Xtr, y)
                proba = clf.predict_proba(Xte)
                sc = route_to_strong_scores(proba, clf.classes_)
                c, q = curve_from_scores(sc, weak_q, weak_c, strong_q, strong_c)
                a = apgr(c, q, oc, oq, weak_q.mean(), strong_q.mean(), weak_c.mean(), strong_c.mean())
                saved = cost_saved_at_quality(c, q, target_q, strong_c.mean())
                print(f"  {kind:8s} {tgt:6s} struct={int(use_struct)}  APGR={a:+.3f}  saved@95%={saved}%")
                if best is None or a > best["apgr"]:
                    best = {"kind": kind, "tgt": tgt, "use_struct": use_struct, "apgr": a,
                            "saved": saved, "clf": clf}

    b = best
    print(f"\nBEST quality head: {b['kind']} {b['tgt']} struct={b['use_struct']}  APGR={b['apgr']:+.3f}  saved@95%={b['saved']}%")
    q_layers, q_classes = (layers_from_logreg(b["clf"]) if b["kind"] == "logistic"
                           else layers_from_mlp(b["clf"]))
    q_classes = [str(c) for c in q_classes]

    # Task head (provenance labels qa/code/math) over embeddings (+struct to match best).
    tl = train["eval_name"].map(task_label)
    mask = tl.notna().to_numpy()
    Xtask = (np.hstack([Etr, Str]) if b["use_struct"] else Etr)[mask]
    task_clf = make_clf("logistic").fit(Xtask, tl[mask].to_numpy())
    # task accuracy on test rows that have provenance labels
    tlt = test["eval_name"].map(task_label); tmask = tlt.notna().to_numpy()
    Xtt = (np.hstack([Ete, Ste]) if b["use_struct"] else Ete)[tmask]
    tacc = (task_clf.predict(Xtt) == tlt[tmask].to_numpy()).mean()
    print(f"task head accuracy (provenance, held-out): {tacc:.3f}")
    t_layers, t_classes = layers_from_logreg(task_clf)
    t_classes = [str(c) for c in t_classes]

    spec = {
        "version": 1,
        "embedding_model": "bge-m3",
        "text_dim": int(Etr.shape[1]),
        "use_struct": bool(b["use_struct"]),
        "struct": {"features": STRUCT_FEATURES, "ctx_token_scale": CTX_TOKEN_SCALE,
                   "n_files_scale": N_FILES_SCALE},
        "quality": {"target": b["tgt"], "classes": q_classes, "layers": q_layers},
        "task": {"classes": t_classes, "layers": t_layers},
        # capability groups + the tier the quality head maps onto
        "groups": ["chat", "code", "reasoning", "long_context", "general"],
        "metrics": {"apgr": round(float(b["apgr"]), 4), "cost_saved_at_95pct_strong": b["saved"],
                    "task_accuracy": round(float(tacc), 4)},
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    json.dump(spec, open(OUT, "w"))
    json.dump(spec, open(os.path.join(HERE, "results", "router_head_spec.json"), "w"), indent=2)
    print(f"\nwrote {OUT}")


if __name__ == "__main__":
    main()
