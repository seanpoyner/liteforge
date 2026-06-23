#!/usr/bin/env python3
"""Train the fusion mapper: (signal one-hots ++ structured features) -> capability group.

This is the "matrix" that maps the panel of expert signals plus codebase-context
features to the final model group. A single linear layer (softmax) is used, so the
learned weights are literally a [features x groups] matrix, exported to fusion.json
for a portable serving forward pass. The fusion consumes expert probabilities at
inference; it is trained on the labels (probabilities collapse to one-hot for the
near-perfect experts).

    python scripts/train_panel_fusion.py --data panel.jsonl --out fusion.json
"""
import argparse
import json
import random

import torch
import torch.nn as nn

from panel_features import (CTX_TOKEN_SCALE, N_FILES_SCALE, STRUCT_FEATURES,
                            extract_features, norm_struct)

SIGNALS = ["task_type", "difficulty", "reasoning_depth", "context_demand"]


def load(path):
    return [json.loads(l) for l in open(path) if l.strip()]


def build_vector(row, orders):
    v = []
    for sig in SIGNALS:
        classes = orders[sig]
        oh = [0.0] * len(classes)
        oh[classes.index(row[sig])] = 1.0
        v += oh
    # Re-parse features from text so training matches serving exactly.
    v += norm_struct(extract_features(row["text"]))
    return v


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", required=True)
    ap.add_argument("--out", default="fusion.json")
    ap.add_argument("--epochs", type=int, default=300)
    ap.add_argument("--lr", type=float, default=0.05)
    ap.add_argument("--seed", type=int, default=21)
    args = ap.parse_args()

    torch.manual_seed(args.seed)
    rows = load(args.data)
    orders = {s: sorted({r[s] for r in rows}) for s in SIGNALS}
    groups = sorted({r["group"] for r in rows})
    g2i = {g: i for i, g in enumerate(groups)}

    X = torch.tensor([build_vector(r, orders) for r in rows], dtype=torch.float32)
    y = torch.tensor([g2i[r["group"]] for r in rows], dtype=torch.long)

    idx = list(range(len(rows)))
    random.Random(args.seed).shuffle(idx)
    ntr = int(len(idx) * 0.85)
    tr, te = idx[:ntr], idx[ntr:]
    Xtr, ytr, Xte, yte = X[tr], y[tr], X[te], y[te]

    in_dim = X.shape[1]
    model = nn.Linear(in_dim, len(groups))
    opt = torch.optim.Adam(model.parameters(), lr=args.lr)
    lossf = nn.CrossEntropyLoss()
    for epoch in range(args.epochs):
        opt.zero_grad()
        loss = lossf(model(Xtr), ytr)
        loss.backward(); opt.step()
        if epoch % 50 == 0 or epoch == args.epochs - 1:
            with torch.no_grad():
                acc = (model(Xte).argmax(-1) == yte).float().mean().item()
            print(f"epoch {epoch:3d}  loss {loss.item():.4f}  test_acc {acc:.4f}")

    with torch.no_grad():
        acc = (model(Xte).argmax(-1) == yte).float().mean().item()
    print(f"\nFusion TEST accuracy: {acc:.4f}  (in_dim={in_dim}, groups={groups})")

    W = model.weight.detach().tolist()   # [groups x in_dim]
    b = model.bias.detach().tolist()     # [groups]
    out = {
        "signals": SIGNALS,
        "signal_classes": orders,
        "struct_features": STRUCT_FEATURES,
        "ctx_token_scale": CTX_TOKEN_SCALE,
        "n_files_scale": N_FILES_SCALE,
        "groups": groups,
        "in_dim": in_dim,
        "W": W,
        "b": b,
    }
    json.dump(out, open(args.out, "w"))
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
