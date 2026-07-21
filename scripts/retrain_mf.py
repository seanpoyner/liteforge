#!/usr/bin/env python3
"""Retrain the RouteLLM-style matrix-factorization (MF) router for a local
embedding model and export weights for LiteForge's native Rust forward pass.

Why: RouteLLM's published MF checkpoint is bound to OpenAI text-embedding-3-small
(1536-dim). To route with a local embedding model served via LiteLLM (e.g. bge-m3,
1024-dim) the MF weights must be trained in that vector space. This script embeds
preference data through your LiteLLM gateway, trains an MF model whose forward pass
matches `crates/liteforge/src/model_routing/mf/forward.rs`, and exports
`mf_weights.json` (schema version 1).

Run on a GPU host (e.g. hal-9000):

    pip install torch requests tqdm
    export LITEFORGE_API_KEY=...    # LiteLLM key
    python scripts/retrain_mf.py \
        --data data/arena_pref.jsonl \
        --litellm-base-url https://litellm.poyner.ai/v1 \
        --embedding-model bge-m3 --dimensions 1024 \
        --out mf_weights.json

Input data: JSONL with one object per line:
    {"prompt": "<user prompt>", "label": 1}
where label = 1 means "the strong model was needed" (strong beat weak / the weak
model's answer was judged insufficient) and 0 means "the weak model sufficed".
Derive this from RouteLLM's released Arena preference data (lm-sys/RouteLLM): for a
chosen strong/weak anchor pair, label = 1 when the strong model wins the battle.

The exported forward pass (must stay in sync with forward.rs):
    pe          = use_proj ? proj_w^T . e + proj_b : e
    logits_m    = cls_w^T . (normalize(row_m) (*) pe) + cls_b      for m in {strong, weak}
    hardness    = sigmoid(logits_strong[strong_class] - logits_weak[weak_class])
"""
import argparse
import json
import os
import sys
import time

try:
    import torch
    import torch.nn as nn
except ImportError:
    sys.exit("torch is required: pip install torch")

try:
    import requests
except ImportError:
    sys.exit("requests is required: pip install requests")


def embed_batch(base_url, api_key, model, dimensions, texts, max_retries=4):
    """Embed a batch of texts via an OpenAI-compatible /embeddings endpoint."""
    url = base_url.rstrip("/") + "/embeddings"
    headers = {"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"}
    payload = {"model": model, "input": texts, "dimensions": dimensions}
    for attempt in range(max_retries):
        resp = requests.post(url, headers=headers, json=payload, timeout=120)
        if resp.status_code == 200:
            data = resp.json()["data"]
            return [d["embedding"] for d in sorted(data, key=lambda d: d["index"])]
        if resp.status_code in (429, 500, 502, 503):
            time.sleep(2 ** attempt)
            continue
        resp.raise_for_status()
    raise RuntimeError(f"embedding request failed after {max_retries} retries")


def load_dataset(path):
    prompts, labels = [], []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            obj = json.loads(line)
            prompts.append(obj["prompt"])
            labels.append(int(obj["label"]))
    if not prompts:
        sys.exit(f"no rows in {path}")
    return prompts, labels


class MFModel(nn.Module):
    """Two-anchor MF matching the Rust forward pass."""

    def __init__(self, text_dim, d, num_classes, use_proj):
        super().__init__()
        self.use_proj = use_proj
        self.strong_row = nn.Parameter(torch.randn(d) * 0.1)
        self.weak_row = nn.Parameter(torch.randn(d) * 0.1)
        if use_proj:
            self.proj = nn.Linear(text_dim, d, bias=True)
        self.classifier = nn.Linear(d, num_classes, bias=True)
        self.strong_class = 0
        self.weak_class = 1

    def _logit(self, row, pe):
        row_n = nn.functional.normalize(row.unsqueeze(0), p=2, dim=1)  # [1, d]
        interaction = row_n * pe  # [B, d]
        return self.classifier(interaction)  # [B, num_classes]

    def forward(self, e):
        pe = self.proj(e) if self.use_proj else e
        ls = self._logit(self.strong_row, pe)[:, self.strong_class]
        lw = self._logit(self.weak_row, pe)[:, self.weak_class]
        return ls - lw  # logit of "strong needed"


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--data", required=True, help="JSONL with {prompt, label}")
    ap.add_argument("--litellm-base-url", default=os.environ.get("LITEFORGE_BASE_URL", "https://litellm.poyner.ai/v1"))
    ap.add_argument("--api-key", default=os.environ.get("LITEFORGE_API_KEY", ""))
    ap.add_argument("--embedding-model", default="bge-m3")
    ap.add_argument("--dimensions", type=int, default=1024)
    ap.add_argument("--d", type=int, default=128, help="latent dimension")
    ap.add_argument("--no-proj", action="store_true", help="disable projection (requires d == dimensions)")
    ap.add_argument("--epochs", type=int, default=30)
    ap.add_argument("--lr", type=float, default=1e-2)
    ap.add_argument("--batch-embed", type=int, default=64)
    ap.add_argument("--cache", default=None, help="optional npy cache for embeddings")
    ap.add_argument("--strong-anchor", default="strong", help="metadata only")
    ap.add_argument("--weak-anchor", default="weak", help="metadata only")
    ap.add_argument("--out", default="mf_weights.json")
    args = ap.parse_args()

    if not args.api_key:
        sys.exit("set --api-key or LITEFORGE_API_KEY")
    use_proj = not args.no_proj
    if not use_proj and args.d != args.dimensions:
        sys.exit("--no-proj requires --d == --dimensions")

    prompts, labels = load_dataset(args.data)
    print(f"loaded {len(prompts)} examples")

    # Embed (with optional disk cache).
    if args.cache and os.path.exists(args.cache):
        import numpy as np
        X = np.load(args.cache)
        print(f"loaded cached embeddings {X.shape}")
        X = torch.tensor(X, dtype=torch.float32)
    else:
        vecs = []
        for i in range(0, len(prompts), args.batch_embed):
            chunk = prompts[i : i + args.batch_embed]
            vecs.extend(embed_batch(args.litellm_base_url, args.api_key,
                                    args.embedding_model, args.dimensions, chunk))
            print(f"  embedded {min(i + args.batch_embed, len(prompts))}/{len(prompts)}", end="\r")
        print()
        X = torch.tensor(vecs, dtype=torch.float32)
        if X.shape[1] != args.dimensions:
            sys.exit(f"embedding dim {X.shape[1]} != --dimensions {args.dimensions}")
        if args.cache:
            import numpy as np
            np.save(args.cache, X.numpy())

    y = torch.tensor(labels, dtype=torch.float32)
    device = "cuda" if torch.cuda.is_available() else "cpu"
    X, y = X.to(device), y.to(device)

    model = MFModel(args.dimensions, args.d, num_classes=2, use_proj=use_proj).to(device)
    opt = torch.optim.Adam(model.parameters(), lr=args.lr)
    loss_fn = nn.BCEWithLogitsLoss()

    model.train()
    for epoch in range(args.epochs):
        opt.zero_grad()
        logits = model(X)
        loss = loss_fn(logits, y)
        loss.backward()
        opt.step()
        if epoch % 5 == 0 or epoch == args.epochs - 1:
            with torch.no_grad():
                acc = ((torch.sigmoid(logits) > 0.5).float() == y).float().mean().item()
            print(f"epoch {epoch:3d}  loss {loss.item():.4f}  acc {acc:.3f}")

    # Export in the exact schema forward.rs / weights.rs expect.
    sd = {k: v.detach().cpu() for k, v in model.state_dict().items()}
    out = {
        "version": 1,
        "embedding_model": args.embedding_model,
        "text_dim": args.dimensions,
        "d": args.d,
        "num_classes": 2,
        "strong_row": sd["strong_row"].tolist(),
        "weak_row": sd["weak_row"].tolist(),
        "use_proj": use_proj,
        # nn.Linear weight is [out, in] = [d, text_dim]; the Rust matvec expects
        # row-major [text_dim * d] computing out[j] = sum_i e[i] * w[i*d + j],
        # i.e. the transpose flattened row-major -> use weight.t().flatten().
        "proj_w": (sd["proj.weight"].t().flatten().tolist() if use_proj else None),
        "proj_b": (sd["proj.bias"].tolist() if use_proj else None),
        # classifier weight [num_classes, d] -> transpose to [d, num_classes] row-major.
        "cls_w": sd["classifier.weight"].t().flatten().tolist(),
        "cls_b": sd["classifier.bias"].tolist(),
        "strong_class": 0,
        "weak_class": 1,
    }
    with open(args.out, "w") as f:
        json.dump(out, f)
    print(f"wrote {args.out}  (d={args.d}, text_dim={args.dimensions}, use_proj={use_proj})")
    print("Point selector.weights_path (or FORGE_ROUTER_WEIGHTS) at this file.")


if __name__ == "__main__":
    main()
