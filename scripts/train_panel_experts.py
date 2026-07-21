#!/usr/bin/env python3
"""Train an independent tiny-BERT expert per routing signal.

Each expert is a separate fine-tune of a small encoder (default bert-tiny) that
predicts ONE signal from the prompt text:
    task_type, difficulty, reasoning_depth, context_demand

Saves each to <out>/<signal>/ with a label map, and reports held-out accuracy.

    python scripts/train_panel_experts.py --data panel.jsonl --out panel-experts
"""
import argparse
import json
import os
import random

import torch
from torch.utils.data import DataLoader, TensorDataset
from transformers import AutoModelForSequenceClassification, AutoTokenizer

SIGNALS = ["task_type", "difficulty", "reasoning_depth", "context_demand"]


def load(path):
    return [json.loads(l) for l in open(path) if l.strip()]


def split(rows, seed, val=0.1, test=0.1):
    idx = list(range(len(rows)))
    random.Random(seed).shuffle(idx)
    n = len(idx)
    nt, nv = int(n * test), int(n * val)
    return idx[nt + nv:], idx[nt:nt + nv], idx[:nt]


def encode(tok, texts, labels, max_len):
    enc = tok(texts, truncation=True, padding="max_length", max_length=max_len, return_tensors="pt")
    return TensorDataset(enc["input_ids"], enc["attention_mask"], torch.tensor(labels))


@torch.no_grad()
def evaluate(model, loader, device, nlab):
    model.eval()
    correct = total = 0
    cm = [[0] * nlab for _ in range(nlab)]
    for ids, mask, y in loader:
        logits = model(input_ids=ids.to(device), attention_mask=mask.to(device)).logits
        for p, t in zip(logits.argmax(-1).cpu().tolist(), y.tolist()):
            cm[t][p] += 1
            correct += int(p == t)
            total += 1
    return correct / max(total, 1), cm


def train_signal(rows, tr_i, va_i, te_i, signal, args, device):
    classes = sorted({r[signal] for r in rows})
    c2i = {c: i for i, c in enumerate(classes)}
    texts = [r["text"] for r in rows]
    labels = [c2i[r[signal]] for r in rows]
    pick = lambda ix: ([texts[i] for i in ix], [labels[i] for i in ix])
    trx, try_ = pick(tr_i); vax, vay = pick(va_i); tex, tey = pick(te_i)

    tok = AutoTokenizer.from_pretrained(args.model)
    model = AutoModelForSequenceClassification.from_pretrained(
        args.model, num_labels=len(classes),
        id2label={i: c for c, i in c2i.items()}, label2id=c2i,
    ).to(device)
    tr = DataLoader(encode(tok, trx, try_, args.max_len), batch_size=args.batch, shuffle=True)
    va = DataLoader(encode(tok, vax, vay, args.max_len), batch_size=args.batch)
    te = DataLoader(encode(tok, tex, tey, args.max_len), batch_size=args.batch)
    opt = torch.optim.AdamW(model.parameters(), lr=args.lr)
    nparams = sum(p.numel() for p in model.parameters())

    print(f"\n=== expert: {signal}  classes={classes}  params={nparams/1e6:.2f}M ===")
    for epoch in range(args.epochs):
        model.train()
        tot = 0.0
        for ids, mask, y in tr:
            opt.zero_grad()
            out = model(input_ids=ids.to(device), attention_mask=mask.to(device), labels=y.to(device))
            out.loss.backward(); opt.step(); tot += out.loss.item()
        vacc, _ = evaluate(model, va, device, len(classes))
        print(f"  epoch {epoch+1}/{args.epochs}  loss {tot/len(tr):.4f}  val_acc {vacc:.4f}")
    tacc, cm = evaluate(model, te, device, len(classes))
    print(f"  TEST acc {tacc:.4f}")
    for i, c in enumerate(classes):
        tot = sum(cm[i]) or 1
        print(f"    {c:14s} recall={cm[i][i]/tot:.4f}  (n={tot})")

    outdir = os.path.join(args.out, signal)
    model.save_pretrained(outdir); tok.save_pretrained(outdir)
    json.dump({"classes": classes}, open(os.path.join(outdir, "labels.json"), "w"))
    return signal, tacc


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", required=True)
    ap.add_argument("--model", default="prajjwal1/bert-tiny")
    ap.add_argument("--out", default="panel-experts")
    ap.add_argument("--epochs", type=int, default=6)
    ap.add_argument("--batch", type=int, default=64)
    ap.add_argument("--lr", type=float, default=5e-4)
    ap.add_argument("--max-len", type=int, default=96)
    ap.add_argument("--seed", type=int, default=21)
    args = ap.parse_args()

    torch.manual_seed(args.seed)
    device = "cuda" if torch.cuda.is_available() else "cpu"
    rows = load(args.data)
    tr_i, va_i, te_i = split(rows, args.seed)
    print(f"rows={len(rows)} train={len(tr_i)} val={len(va_i)} test={len(te_i)} device={device}")
    os.makedirs(args.out, exist_ok=True)

    results = [train_signal(rows, tr_i, va_i, te_i, s, args, device) for s in SIGNALS]
    print("\n=== summary ===")
    for s, a in results:
        print(f"  {s:16s} test_acc={a:.4f}")


if __name__ == "__main__":
    main()
