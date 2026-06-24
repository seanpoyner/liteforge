#!/usr/bin/env python3
"""Fetch RouterBench and derive labeled evaluation subsets.

RouterBench (withmartian/routerbench) ships a pandas pickle: per prompt, a 0/1
correctness score and a cost for each of 11 LLMs, an `eval_name` provenance, and an
oracle column. We reuse it for everything:

  * Intrinsic eval: map `eval_name` -> expected task_type, and derive a difficulty
    proxy from which model tiers solve the prompt.
  * Extrinsic eval: per-model score + cost + oracle for the cost-quality frontier.

Outputs (under scripts/eval/data/):
  routerbench/routerbench_0shot.pkl   (raw)
  intrinsic_sample.jsonl              ({prompt, eval_name, task_label, difficulty_proxy})
"""
import json
import os
import random

import pandas as pd
from huggingface_hub import hf_hub_download

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")

# RouterBench's 11 models grouped into coarse capability tiers (by avg acc/cost).
WEAK = "mistralai/mixtral-8x7b-chat"
MID = "zero-one-ai/Yi-34B-Chat"
STRONG = "gpt-4-1106-preview"
TIER_MODEL = {"weak": WEAK, "mid": MID, "strong": STRONG}

# eval_name (provenance) -> expected task_type label (subset we can label cleanly).
def task_label(eval_name: str):
    e = eval_name.lower()
    if any(k in e for k in ["math", "gsm", "arith", "remainder", "theorem"]):
        return "math"
    if any(k in e for k in ["mbpp", "humaneval", "code", "program"]):
        return "code"
    if any(k in e for k in ["mmlu", "hellaswag", "winogrande", "arc", "truthful",
                            "openbook", "qa", "trivia", "race"]):
        return "qa"
    return None  # unlabeled (e.g. mt-bench, chinese_* niche sets) -> skip for task eval


def difficulty_proxy(row):
    """Derive easy/medium/hard from which tier solves the prompt.

    easy  : the weak model is already correct
    hard  : only the strong model is correct (weak and mid both fail)
    medium: otherwise (mid solves it, or mixed)
    """
    w = float(row[WEAK]); m = float(row[MID]); s = float(row[STRONG])
    if w >= 0.5:
        return "easy"
    if s >= 0.5 and m < 0.5:
        return "hard"
    if m >= 0.5:
        return "medium"
    # weak fails, mid fails, strong fails -> treat as hard (needs the best we have)
    return "hard"


def load_routerbench():
    p = hf_hub_download("withmartian/routerbench", "routerbench_0shot.pkl",
                        repo_type="dataset", local_dir=os.path.join(DATA, "routerbench"))
    return pd.read_pickle(p)


def main():
    os.makedirs(DATA, exist_ok=True)
    df = load_routerbench()
    print(f"RouterBench: {df.shape[0]} prompts, {df['eval_name'].nunique()} source evals")

    # Build a balanced intrinsic sample with task + difficulty labels.
    rng = random.Random(13)
    rows = []
    for _, r in df.iterrows():
        tl = task_label(r["eval_name"])
        if tl is None:
            continue
        rows.append({
            "prompt": str(r["prompt"])[:4000],
            "eval_name": r["eval_name"],
            "task_label": tl,
            "difficulty_proxy": difficulty_proxy(r),
        })
    rng.shuffle(rows)

    # Cap per task_label for a fast, balanced intrinsic set.
    cap = 800
    counts = {}
    sample = []
    for r in rows:
        c = counts.get(r["task_label"], 0)
        if c < cap:
            sample.append(r); counts[r["task_label"]] = c + 1
    out = os.path.join(DATA, "intrinsic_sample.jsonl")
    with open(out, "w") as f:
        for r in sample:
            f.write(json.dumps(r) + "\n")
    from collections import Counter
    print("intrinsic sample:", len(sample),
          "| task:", dict(Counter(r["task_label"] for r in sample)),
          "| difficulty:", dict(Counter(r["difficulty_proxy"] for r in sample)))
    print("wrote", out)


if __name__ == "__main__":
    main()
