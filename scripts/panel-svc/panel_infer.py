"""Panel router inference: 4 tiny BERT experts + structured features + fusion matrix.

Loads the four independent expert classifiers, runs them on the prompt+context,
extracts structured codebase-context features, and applies the learned fusion
matrix to produce capability-group scores. Pure CPU; the models are tiny.
"""
import json
import math
import os

import torch
import torch.nn.functional as F
from transformers import AutoModelForSequenceClassification, AutoTokenizer

from panel_features import extract_features, norm_struct

SIGNALS = ["task_type", "difficulty", "reasoning_depth", "context_demand"]


class Panel:
    def __init__(self, base_dir):
        self.experts = {}
        self.tok = {}
        self.classes = {}
        for sig in SIGNALS:
            d = os.path.join(base_dir, sig)
            self.tok[sig] = AutoTokenizer.from_pretrained(d)
            m = AutoModelForSequenceClassification.from_pretrained(d)
            m.eval()
            self.experts[sig] = m
            self.classes[sig] = json.load(open(os.path.join(d, "labels.json")))["classes"]
        self.fusion = json.load(open(os.path.join(base_dir, "fusion.json")))
        # Sanity: expert class order must match fusion's expected order.
        for sig in SIGNALS:
            assert self.classes[sig] == self.fusion["signal_classes"][sig], \
                f"class order mismatch for {sig}"
        self.W = self.fusion["W"]            # [groups x in_dim]
        self.b = self.fusion["b"]            # [groups]
        self.groups = self.fusion["groups"]

    @torch.no_grad()
    def _signal_probs(self, sig, text):
        enc = self.tok[sig](text, truncation=True, padding=True, max_length=96, return_tensors="pt")
        logits = self.experts[sig](**enc).logits
        return F.softmax(logits, dim=-1)[0].tolist()

    def classify(self, text):
        # Per-expert probability vectors (already in fusion's class order).
        signal_probs = {sig: self._signal_probs(sig, text) for sig in SIGNALS}
        feats = extract_features(text)
        # Build the fusion input vector: concat(expert probs in SIGNALS order) ++ norm features.
        x = []
        for sig in SIGNALS:
            x += signal_probs[sig]
        x += norm_struct(feats)
        assert len(x) == self.fusion["in_dim"], (len(x), self.fusion["in_dim"])
        # Linear forward: logits[g] = sum_i W[g][i]*x[i] + b[g]
        logits = [sum(self.W[g][i] * x[i] for i in range(len(x))) + self.b[g]
                  for g in range(len(self.groups))]
        m = max(logits)
        exps = [math.exp(l - m) for l in logits]
        s = sum(exps)
        scores = {self.groups[g]: round(exps[g] / s, 6) for g in range(len(self.groups))}
        # Human-readable top signal labels for observability.
        top_signals = {sig: self.classes[sig][max(range(len(p)), key=lambda i: p[i])]
                       for sig, p in signal_probs.items()}
        return scores, top_signals, feats
