"""Python mirror of the Rust MF forward pass
(crates/liteforge/src/model_routing/mf/forward.rs), for offline evaluation.

Given the JSON MF weights and a prompt embedding e (text_dim), returns the scalar
"hardness" in (0,1): the probability the strong model is needed.
"""
import json
import math


def _normalize(v):
    n = math.sqrt(sum(x * x for x in v))
    return [x / n for x in v] if n > 0 else [0.0 for _ in v]


def _matvec(m, rows, cols, v, bias):
    out = list(bias)
    for i in range(rows):
        vi = v[i]
        if vi == 0.0:
            continue
        base = i * cols
        for j in range(cols):
            out[j] += vi * m[base + j]
    return out


def _sigmoid(x):
    if x >= 0:
        return 1.0 / (1.0 + math.exp(-x))
    e = math.exp(x)
    return e / (1.0 + e)


class MF:
    def __init__(self, weights_path):
        self.w = json.load(open(weights_path))

    def hardness(self, e):
        w = self.w
        if len(e) != w["text_dim"]:
            raise ValueError(f"embedding dim {len(e)} != text_dim {w['text_dim']}")
        if w["use_proj"]:
            bias = w.get("proj_b") or [0.0] * w["d"]
            pe = _matvec(w["proj_w"], w["text_dim"], w["d"], e, bias)
        else:
            pe = list(e)
        strong = _normalize(w["strong_row"])
        weak = _normalize(w["weak_row"])

        def logits(anchor):
            inter = [anchor[i] * pe[i] for i in range(w["d"])]
            return _matvec(w["cls_w"], w["d"], w["num_classes"], inter, w["cls_b"])

        ls = logits(strong)
        lw = logits(weak)
        return _sigmoid(ls[w["strong_class"]] - lw[w["weak_class"]])
