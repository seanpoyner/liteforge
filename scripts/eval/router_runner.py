"""Run the routers offline to produce routing signals/decisions for evaluation.

Routers:
  * panel       - the 4-expert + features + fusion panel (local inference)
  * router-bert - the single 3-class difficulty classifier (local inference)
  * mf          - the native RouteLLM MF port (needs bge-m3 embeddings via LiteLLM)

Model artifacts default to ~/.forge/router-models (override with ROUTER_MODELS).
"""
import os
import sys

import torch
import torch.nn.functional as F
from transformers import AutoModelForSequenceClassification, AutoTokenizer

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
# Make the panel inference + shared features importable.
sys.path.insert(0, os.path.join(REPO, "crates"))  # no-op safety
sys.path.insert(0, os.path.join(REPO, "scripts", "panel-svc"))
sys.path.insert(0, os.path.join(REPO, "scripts"))

MODELS = os.environ.get("ROUTER_MODELS", os.path.expanduser("~/.forge/router-models"))


class PanelRunner:
    def __init__(self, base=None):
        from panel_infer import Panel  # from scripts/panel-svc
        self.panel = Panel(base or os.path.join(MODELS, "panel"))

    def signals(self, text):
        _scores, signals, _feats = self.panel.classify(text)
        return signals

    def decision(self, text):
        scores, signals, feats = self.panel.classify(text)
        group = max(scores, key=scores.get)
        return {"group": group, "scores": scores, "signals": signals, "features": feats}


class BertRunner:
    """Single 3-class difficulty classifier (router-bert)."""
    def __init__(self, base=None):
        d = base or os.path.join(MODELS, "router-bert")
        self.tok = AutoTokenizer.from_pretrained(d)
        self.model = AutoModelForSequenceClassification.from_pretrained(d).eval()
        self.id2label = self.model.config.id2label

    @torch.no_grad()
    def probs(self, text):
        enc = self.tok(text, truncation=True, padding=True, max_length=96, return_tensors="pt")
        p = F.softmax(self.model(**enc).logits, dim=-1)[0].tolist()
        return {self.id2label[i]: p[i] for i in range(len(p))}

    def difficulty(self, text):
        p = self.probs(text)
        return max(p, key=p.get)


class MfRunner:
    """Native MF hardness over bge-m3 embeddings (fetched via LiteLLM)."""
    def __init__(self, weights=None, base_url=None, model="bge-m3", dim=1024):
        from mf_forward import MF
        self.mf = MF(weights or os.path.expanduser("~/.forge/mf_weights.bge-m3.json"))
        self.base_url = base_url or os.environ.get("LITEFORGE_BASE_URL", "https://litellm.poyner.ai/v1")
        self.key = os.environ.get("LITEFORGE_API_KEY", "")
        self.model = model
        self.dim = dim

    def embed(self, text):
        import json
        import urllib.request
        body = json.dumps({"model": self.model, "input": text, "dimensions": self.dim}).encode()
        req = urllib.request.Request(self.base_url.rstrip("/") + "/embeddings", data=body,
                                     headers={"Authorization": f"Bearer {self.key}",
                                              "Content-Type": "application/json"})
        d = json.load(urllib.request.urlopen(req, timeout=60))
        return d["data"][0]["embedding"]

    def embed_batch(self, texts):
        import json
        import urllib.request
        body = json.dumps({"model": self.model, "input": texts, "dimensions": self.dim}).encode()
        req = urllib.request.Request(self.base_url.rstrip("/") + "/embeddings", data=body,
                                     headers={"Authorization": f"Bearer {self.key}",
                                              "Content-Type": "application/json"})
        d = json.load(urllib.request.urlopen(req, timeout=120))
        return [r["embedding"] for r in sorted(d["data"], key=lambda x: x["index"])]

    def hardness(self, text):
        return self.mf.hardness(self.embed(text))

    def hardness_batch(self, texts, batch=32):
        out = []
        for i in range(0, len(texts), batch):
            for e in self.embed_batch(texts[i:i + batch]):
                out.append(self.mf.hardness(e))
        return out
