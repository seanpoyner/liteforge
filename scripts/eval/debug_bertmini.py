#!/usr/bin/env python3
"""Settle the bert-mini 'ln 2 plateau': was it a capacity ceiling or a training bug?

The original from-scratch run used LR 5e-4 (25x too high for a BERT fine-tune) with no
warmup and 6 epochs, and its loss never left ln 2 ~= 0.693 (coin-flip cross-entropy).
This re-runs with a correct recipe (LR 2e-5, linear warmup, more epochs, full train data)
and reports the loss curve, held-out accuracy, and RouterBench APGR, so the paper's claim
can be corrected.

    python scripts/eval/debug_bertmini.py
"""
import numpy as np
import pandas as pd
import torch
import torch.nn.functional as F
from torch.utils.data import DataLoader, TensorDataset
from transformers import (AutoModelForSequenceClassification, AutoTokenizer,
                          get_linear_schedule_with_warmup)

from routerbench_eval import (PKL, STRONG, WEAK, apgr, cost_saved_at_quality,
                              curve_from_scores, subsample)


def run(lr, epochs, warmup_frac, tag, max_train=30000):
    device = "cuda" if torch.cuda.is_available() else "cpu"
    df = pd.read_pickle(PKL)
    test = subsample(df, 2000, seed=13)
    train = df[~df["sample_id"].isin(set(test["sample_id"]))]
    train = train.sample(min(len(train), max_train), random_state=13)
    ytr = (train[WEAK].astype(float) >= 0.5).astype(int).to_numpy()  # 1 = weak correct

    tok = AutoTokenizer.from_pretrained("prajjwal1/bert-mini")
    model = AutoModelForSequenceClassification.from_pretrained("prajjwal1/bert-mini", num_labels=2).to(device)
    enc = tok(train["prompt"].astype(str).str.slice(0, 2000).tolist(),
              truncation=True, padding="max_length", max_length=128, return_tensors="pt")
    ds = TensorDataset(enc["input_ids"], enc["attention_mask"], torch.tensor(ytr))
    dl = DataLoader(ds, batch_size=64, shuffle=True)
    opt = torch.optim.AdamW(model.parameters(), lr=lr)
    steps = len(dl) * epochs
    sched = get_linear_schedule_with_warmup(opt, int(steps * warmup_frac), steps)

    print(f"\n[{tag}] lr={lr} epochs={epochs} warmup={warmup_frac} train={len(train)} weak_rate={ytr.mean():.3f}")
    model.train()
    for ep in range(epochs):
        tot = 0.0
        for ids, mask, y in dl:
            opt.zero_grad()
            out = model(input_ids=ids.to(device), attention_mask=mask.to(device), labels=y.to(device))
            out.loss.backward(); opt.step(); sched.step(); tot += out.loss.item()
        print(f"  epoch {ep+1}/{epochs} loss {tot/len(dl):.4f}")

    # held-out: accuracy at predicting weak-correct + RouterBench APGR
    model.eval()
    te = test.reset_index(drop=True)
    weak_q = te[WEAK].astype(float).to_numpy(); strong_q = te[STRONG].astype(float).to_numpy()
    weak_c = te[f"{WEAK}|total_cost"].to_numpy(); strong_c = te[f"{STRONG}|total_cost"].to_numpy()
    yte = (weak_q >= 0.5).astype(int)
    tp = te["prompt"].astype(str).str.slice(0, 2000).tolist()
    probs = []
    with torch.no_grad():
        for i in range(0, len(tp), 128):
            e = tok(tp[i:i+128], truncation=True, padding=True, max_length=128, return_tensors="pt")
            p = F.softmax(model(input_ids=e["input_ids"].to(device),
                                attention_mask=e["attention_mask"].to(device)).logits, dim=-1)
            probs.append(p.cpu().numpy())
    probs = np.concatenate(probs)
    acc = ((probs[:, 1] >= 0.5).astype(int) == yte).mean()
    scores = probs[:, 0]  # route-to-strong = P(weak wrong)
    oc, oq = curve_from_scores(strong_q - weak_q + 1e-6*np.random.RandomState(0).rand(len(te)),
                               weak_q, weak_c, strong_q, strong_c)
    c, q = curve_from_scores(scores, weak_q, weak_c, strong_q, strong_c)
    a = apgr(c, q, oc, oq, weak_q.mean(), strong_q.mean(), weak_c.mean(), strong_c.mean())
    saved = cost_saved_at_quality(c, q, 0.95*strong_q.mean(), strong_c.mean())
    print(f"  [{tag}] held-out acc(weak-correct)={acc:.3f}  APGR={a:+.3f}  saved@95%={saved}%")
    return acc, a


def main():
    print("=== reproduce the bug (original recipe) ===")
    run(5e-4, 6, 0.0, "lr=5e-4 (original)", max_train=12000)
    print("\n=== corrected recipe ===")
    run(2e-5, 10, 0.1, "lr=2e-5 +warmup (fixed)")


if __name__ == "__main__":
    main()
