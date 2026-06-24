#!/usr/bin/env python3
"""Retrain a tiny router on real RouterBench data and re-evaluate.

Trains a bert-tiny binary classifier to predict "route to strong" (label = 1 when
the strong model is correct and the weak model is not, i.e. strong is genuinely
needed) on a RouterBench TRAIN split, then evaluates the cost-quality curve on the
SAME held-out test subsample used by routerbench_eval.py (seed 13), so zero-shot and
retrained numbers are directly comparable.

Writes scripts/eval/results/routerbench_retrained.json.
"""
import json
import os

import numpy as np
import pandas as pd
import torch
import torch.nn.functional as F
from torch.utils.data import DataLoader, TensorDataset
from transformers import AutoModelForSequenceClassification, AutoTokenizer

from routerbench_eval import (PKL, STRONG, WEAK, apgr, cost_saved_at_quality,
                              curve_from_scores, subsample)

HERE = os.path.dirname(os.path.abspath(__file__))
RESULTS = os.path.join(HERE, "results")
SAVE = os.path.expanduser("~/.forge/router-models/router-rb")


def main():
    os.makedirs(RESULTS, exist_ok=True)
    device = "cuda" if torch.cuda.is_available() else "cpu"
    df = pd.read_pickle(PKL)
    test = subsample(df, 2000, seed=13)
    test_ids = set(test["sample_id"])
    train = df[~df["sample_id"].isin(test_ids)].reset_index(drop=True)
    print(f"train={len(train)} test={len(test)} device={device}")

    # Target: predict whether the WEAK model is correct (route-to-strong = P(weak wrong)).
    # This is balanced and directly learnable, mirroring RouteLLM's weak-deferral framing.
    train = train.copy()
    train["y"] = (train[WEAK].astype(float) >= 0.5).astype(int)  # 1 = weak correct
    tr = train.sample(min(len(train), 30000), random_state=13)
    print(f"train rows={len(tr)} weak_correct_rate={tr.y.mean():.3f}")

    model_id = "prajjwal1/bert-mini"  # ~11M params, still tiny, more capacity than bert-tiny
    tok = AutoTokenizer.from_pretrained(model_id)
    model = AutoModelForSequenceClassification.from_pretrained(model_id, num_labels=2).to(device)
    texts = tr["prompt"].astype(str).str.slice(0, 2000).tolist()
    enc = tok(texts, truncation=True, padding="max_length", max_length=128, return_tensors="pt")
    ds = TensorDataset(enc["input_ids"], enc["attention_mask"], torch.tensor(tr["y"].to_numpy()))
    dl = DataLoader(ds, batch_size=64, shuffle=True)
    opt = torch.optim.AdamW(model.parameters(), lr=5e-4)
    n_epochs = 6
    model.train()
    for epoch in range(n_epochs):
        tot = 0.0
        for ids, mask, y in dl:
            opt.zero_grad()
            out = model(input_ids=ids.to(device), attention_mask=mask.to(device), labels=y.to(device))
            out.loss.backward(); opt.step(); tot += out.loss.item()
        print(f"  epoch {epoch+1}/{n_epochs} loss {tot/len(dl):.4f}")
    model.save_pretrained(SAVE); tok.save_pretrained(SAVE)
    print(f"saved retrained router to {SAVE}")

    # Score the held-out test subsample.
    model.eval()
    weak_q = test[WEAK].astype(float).to_numpy(); strong_q = test[STRONG].astype(float).to_numpy()
    weak_c = test[f"{WEAK}|total_cost"].to_numpy(); strong_c = test[f"{STRONG}|total_cost"].to_numpy()
    tp = test["prompt"].astype(str).str.slice(0, 2000).tolist()
    scores = []
    with torch.no_grad():
        for i in range(0, len(tp), 128):
            e = tok(tp[i:i + 128], truncation=True, padding=True, max_length=128, return_tensors="pt")
            # route-to-strong propensity = P(weak is wrong) = column 0
            p = F.softmax(model(input_ids=e["input_ids"].to(device),
                                attention_mask=e["attention_mask"].to(device)).logits, dim=-1)[:, 0]
            scores.extend(p.cpu().tolist())
    scores = np.array(scores)

    oracle_scores = strong_q - weak_q + 1e-6 * np.random.RandomState(0).rand(len(test))
    oc, oq = curve_from_scores(oracle_scores, weak_q, weak_c, strong_q, strong_c)
    c, q = curve_from_scores(scores, weak_q, weak_c, strong_q, strong_c)
    target = 0.95 * strong_q.mean()
    res = {
        "n": len(test), "tag": "retrained",
        "router-rb": {
            "APGR": round(float(apgr(c, q, oc, oq, weak_q.mean(), strong_q.mean(),
                                     weak_c.mean(), strong_c.mean())), 4),
            "cost_saved_at_95pct_strong": cost_saved_at_quality(c, q, target, strong_c.mean()),
        },
        "curves": {"router-rb": {"cost": c.tolist(), "quality": q.tolist()},
                   "oracle": {"cost": oc.tolist(), "quality": oq.tolist()}},
        "weak_quality": round(float(weak_q.mean()), 4), "strong_quality": round(float(strong_q.mean()), 4),
    }
    json.dump(res, open(os.path.join(RESULTS, "routerbench_retrained.json"), "w"))
    print(f"\n=== RouterBench retrained (held-out test) ===")
    print(f"  router-rb APGR={res['router-rb']['APGR']:.3f}  cost_saved@95%strong={res['router-rb']['cost_saved_at_95pct_strong']}%")


if __name__ == "__main__":
    main()
