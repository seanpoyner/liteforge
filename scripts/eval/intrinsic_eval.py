#!/usr/bin/env python3
"""Intrinsic OOD evaluation of the panel experts on real RouterBench prompts.

Tests whether the synthetic-trained task_type and difficulty experts generalize to
real benchmark prompts (MMLU/GSM8K/MBPP/etc.), using provenance-derived labels and a
model-success difficulty proxy from fetch_data.py.

Writes scripts/eval/results/intrinsic.json.
"""
import json
import os

from sklearn.metrics import accuracy_score, confusion_matrix, f1_score

from router_runner import BertRunner, PanelRunner

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data", "intrinsic_sample.jsonl")
RESULTS = os.path.join(HERE, "results")

# Map the panel's 9-way task_type to the 3 eval labels we can ground-truth.
TASK_COARSE = {
    "math": "math",
    "code": "code", "debug": "code", "refactor": "code", "data": "code",
    "qa": "qa", "reasoning": "qa", "writing": "qa", "chitchat": "qa",
}


def cm_dict(y_true, y_pred, labels):
    cm = confusion_matrix(y_true, y_pred, labels=labels)
    return {labels[i]: {labels[j]: int(cm[i][j]) for j in range(len(labels))}
            for i in range(len(labels))}


def main():
    os.makedirs(RESULTS, exist_ok=True)
    rows = [json.loads(l) for l in open(DATA) if l.strip()]
    print(f"loaded {len(rows)} labeled prompts")

    panel = PanelRunner()
    bert = BertRunner()

    task_true, task_pred = [], []
    diff_true, diff_panel, diff_bert = [], [], []
    for i, r in enumerate(rows):
        text = r["prompt"]
        sig = panel.signals(text)
        task_true.append(r["task_label"])
        task_pred.append(TASK_COARSE.get(sig["task_type"], "qa"))
        diff_true.append(r["difficulty_proxy"])
        diff_panel.append(sig["difficulty"])
        diff_bert.append(bert.difficulty(text))
        if (i + 1) % 400 == 0:
            print(f"  {i+1}/{len(rows)}")

    task_labels = ["qa", "code", "math"]
    diff_labels = ["easy", "medium", "hard"]
    res = {
        "n": len(rows),
        "task_type": {
            "accuracy_coarse": round(accuracy_score(task_true, task_pred), 4),
            "macro_f1": round(f1_score(task_true, task_pred, labels=task_labels,
                                       average="macro", zero_division=0), 4),
            "confusion": cm_dict(task_true, task_pred, task_labels),
        },
        "difficulty_panel": {
            "accuracy": round(accuracy_score(diff_true, diff_panel), 4),
            "macro_f1": round(f1_score(diff_true, diff_panel, labels=diff_labels,
                                       average="macro", zero_division=0), 4),
            "confusion": cm_dict(diff_true, diff_panel, diff_labels),
        },
        "difficulty_router_bert": {
            "accuracy": round(accuracy_score(diff_true, diff_bert), 4),
            "macro_f1": round(f1_score(diff_true, diff_bert, labels=diff_labels,
                                       average="macro", zero_division=0), 4),
            "confusion": cm_dict(diff_true, diff_bert, diff_labels),
        },
    }
    json.dump(res, open(os.path.join(RESULTS, "intrinsic.json"), "w"), indent=2)

    print("\n=== INTRINSIC (out-of-distribution) ===")
    print(f"task_type coarse acc : {res['task_type']['accuracy_coarse']}  macroF1 {res['task_type']['macro_f1']}")
    print(f"  confusion {res['task_type']['confusion']}")
    print(f"difficulty (panel)   : acc {res['difficulty_panel']['accuracy']}  macroF1 {res['difficulty_panel']['macro_f1']}")
    print(f"difficulty (rbert)   : acc {res['difficulty_router_bert']['accuracy']}  macroF1 {res['difficulty_router_bert']['macro_f1']}")
    print("wrote results/intrinsic.json")


if __name__ == "__main__":
    main()
