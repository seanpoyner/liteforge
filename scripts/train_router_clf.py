#!/usr/bin/env python3
"""Fine-tune a small (sub-1B, ideally tiny) encoder as a 3-class routing
classifier (easy/medium/hard -> cheap/balanced/premium).

Dependency-light: transformers + torch only (no `datasets`). Reports held-out
test accuracy and per-class precision/recall, then saves model + tokenizer.

    python scripts/train_router_clf.py --data clf_data.jsonl \
        --model prajjwal1/bert-tiny --out router-clf --epochs 6

Smaller backbones to try (pick the smallest that meets your accuracy bar):
    prajjwal1/bert-tiny   (~4.4M)
    prajjwal1/bert-mini   (~11M)
    sentence-transformers/all-MiniLM-L6-v2 (~22M)
"""
import argparse
import json
import random

import torch
from torch.utils.data import DataLoader, TensorDataset
from transformers import AutoModelForSequenceClassification, AutoTokenizer

LABELS = ["easy", "medium", "hard"]


def load(path):
    rows = [json.loads(l) for l in open(path) if l.strip()]
    return [r["text"] for r in rows], [int(r["label"]) for r in rows]


def split(texts, labels, seed, val=0.1, test=0.1):
    idx = list(range(len(texts)))
    random.Random(seed).shuffle(idx)
    n = len(idx)
    n_test = int(n * test)
    n_val = int(n * val)
    te, va, tr = idx[:n_test], idx[n_test:n_test + n_val], idx[n_test + n_val:]
    pick = lambda ix: ([texts[i] for i in ix], [labels[i] for i in ix])
    return pick(tr), pick(va), pick(te)


def encode(tok, texts, labels, max_len):
    enc = tok(texts, truncation=True, padding="max_length", max_length=max_len, return_tensors="pt")
    return TensorDataset(enc["input_ids"], enc["attention_mask"], torch.tensor(labels))


@torch.no_grad()
def evaluate(model, loader, device):
    model.eval()
    correct = total = 0
    cm = [[0] * 3 for _ in range(3)]
    for ids, mask, y in loader:
        ids, mask = ids.to(device), mask.to(device)
        logits = model(input_ids=ids, attention_mask=mask).logits
        pred = logits.argmax(-1).cpu()
        for p, t in zip(pred.tolist(), y.tolist()):
            cm[t][p] += 1
            correct += int(p == t)
            total += 1
    acc = correct / max(total, 1)
    return acc, cm


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", required=True)
    ap.add_argument("--model", default="prajjwal1/bert-tiny")
    ap.add_argument("--out", default="router-clf")
    ap.add_argument("--epochs", type=int, default=6)
    ap.add_argument("--batch", type=int, default=64)
    ap.add_argument("--lr", type=float, default=5e-4)
    ap.add_argument("--max-len", type=int, default=64)
    ap.add_argument("--seed", type=int, default=13)
    args = ap.parse_args()

    torch.manual_seed(args.seed)
    device = "cuda" if torch.cuda.is_available() else "cpu"
    texts, labels = load(args.data)
    (trx, try_), (vax, vay), (tex, tey) = split(texts, labels, args.seed)
    print(f"train={len(trx)} val={len(vax)} test={len(tex)} device={device} model={args.model}")

    tok = AutoTokenizer.from_pretrained(args.model)
    model = AutoModelForSequenceClassification.from_pretrained(
        args.model, num_labels=3,
        id2label={i: l for i, l in enumerate(LABELS)},
        label2id={l: i for i, l in enumerate(LABELS)},
    ).to(device)

    tr = DataLoader(encode(tok, trx, try_, args.max_len), batch_size=args.batch, shuffle=True)
    va = DataLoader(encode(tok, vax, vay, args.max_len), batch_size=args.batch)
    te = DataLoader(encode(tok, tex, tey, args.max_len), batch_size=args.batch)

    opt = torch.optim.AdamW(model.parameters(), lr=args.lr)
    n_params = sum(p.numel() for p in model.parameters())
    print(f"params={n_params/1e6:.2f}M")

    for epoch in range(args.epochs):
        model.train()
        tot = 0.0
        for ids, mask, y in tr:
            ids, mask, y = ids.to(device), mask.to(device), y.to(device)
            opt.zero_grad()
            out = model(input_ids=ids, attention_mask=mask, labels=y)
            out.loss.backward()
            opt.step()
            tot += out.loss.item()
        vacc, _ = evaluate(model, va, device)
        print(f"epoch {epoch+1}/{args.epochs}  loss {tot/len(tr):.4f}  val_acc {vacc:.4f}")

    tacc, cm = evaluate(model, te, device)
    print(f"\nTEST accuracy: {tacc:.4f}  ({sum(cm[i][i] for i in range(3))}/{sum(sum(r) for r in cm)})")
    print("confusion matrix (rows=true, cols=pred) [easy, medium, hard]:")
    for i, row in enumerate(cm):
        tot = sum(row) or 1
        recall = row[i] / tot
        print(f"  {LABELS[i]:7s} {row}  recall={recall:.4f}")

    model.save_pretrained(args.out)
    tok.save_pretrained(args.out)
    print(f"\nsaved model + tokenizer to {args.out}/  ({n_params/1e6:.2f}M params)")


if __name__ == "__main__":
    main()
