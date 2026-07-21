"""Embedding-head inference for the router-panel service (parity with the native
Rust EmbeddingHeadSelector). Embeds via LiteLLM bge-m3, runs the learned quality +
task heads from router-head.json, and fuses into capability-group scores. numpy only
(no torch): the heads are a couple of matrix multiplies.
"""
import json
import os

import numpy as np
import requests

from panel_features import extract_features, norm_struct

HEAD_PATH = os.environ.get("ROUTER_HEAD", "/app/router-head.json")
BASE = os.environ.get("LITEFORGE_BASE_URL", "https://litellm.poyner.ai/v1")
KEY = os.environ.get("LITEFORGE_API_KEY", "")


def _forward(layers, x):
    v = np.asarray(x, dtype=np.float32)
    for layer in layers:
        W = np.asarray(layer["W"], dtype=np.float32)   # [out, in]
        b = np.asarray(layer["b"], dtype=np.float32)
        v = W @ v + b
        if layer.get("activation") == "relu":
            v = np.maximum(v, 0.0)
    e = np.exp(v - v.max())
    return e / e.sum()


def _hardness(quality, probs):
    cls = quality["classes"]
    if "strong" in cls:
        h = probs[cls.index("strong")]
        if "mid" in cls:
            h += 0.5 * probs[cls.index("mid")]
        return float(min(max(h, 0.0), 1.0))
    if "0" in cls:
        return float(min(max(probs[cls.index("0")], 0.0), 1.0))
    return float(probs[-1])


def _fuse(h, task, feats, ctx_norm):
    context_high = feats["n_files"] >= 4 or ctx_norm >= 0.6
    is_code = task == "code" or feats["has_code"] or feats["has_diff"]
    trivial = feats["ctx_tokens"] < 8 and not feats["has_code"] and not feats["has_diff"] and not feats["has_error"]
    scores = {
        "code": 0.9 if is_code else 0.1,
        "reasoning": (0.55 + 0.45 * h) if h > 0.6 else 0.1 * h,
        "chat": 0.95 if trivial else 0.1,
        "long_context": 0.82 if context_high else 0.1,
        "general": 0.5,
    }
    return {k: round(float(v), 6) for k, v in scores.items()}


class EmbeddingHead:
    def __init__(self):
        self.spec = json.load(open(HEAD_PATH))

    def embed(self, text):
        body = {"model": self.spec["embedding_model"], "input": text, "dimensions": self.spec["text_dim"]}
        r = requests.post(BASE.rstrip("/") + "/embeddings", json=body,
                          headers={"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"},
                          timeout=60)
        r.raise_for_status()
        return r.json()["data"][0]["embedding"]

    def classify(self, text):
        emb = self.embed(text)
        feats = extract_features(text)
        ns = norm_struct(feats)
        x = list(emb) + list(ns) if self.spec["use_struct"] else list(emb)
        qprobs = _forward(self.spec["quality"]["layers"], x)
        h = _hardness(self.spec["quality"], qprobs)
        tprobs = _forward(self.spec["task"]["layers"], x)
        task = self.spec["task"]["classes"][int(np.argmax(tprobs))]
        scores = _fuse(h, task, feats, ns[0])
        signals = {"hardness": round(h, 4), "task": task}
        return scores, signals, feats
