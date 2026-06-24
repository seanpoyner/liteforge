#!/usr/bin/env python3
"""Render figures for the paper from the results JSON.

  results/figures/cost_quality.png       - cost-quality frontier (routers vs oracle/random)
  results/figures/intrinsic_confusion.png - task + difficulty confusion (OOD)
"""
import json
import os

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
RES = os.path.join(HERE, "results")
FIG = os.path.join(RES, "figures")

# Dracula accents.
COL = {"oracle": "#16A34A", "random": "#6272A4", "panel": "#BD93F9",
       "router-bert": "#FF79C6", "mf": "#FFB86C", "router-rb": "#0E7490"}


def cost_quality():
    zs = json.load(open(os.path.join(RES, "routerbench_zeroshot.json")))
    def maybe(name):
        p = os.path.join(RES, name)
        return json.load(open(p)) if os.path.exists(p) else None
    rt = maybe("routerbench_retrained.json")        # router-rb (from scratch)
    emb = maybe("routerbench_retrained_emb.json")   # router-emb (embedding head)

    plt.figure(figsize=(7, 5))
    oc = zs["curves"]["oracle"]
    plt.plot(oc["cost"], oc["quality"], color=COL["oracle"], lw=2.2, label="oracle", zorder=5)
    # random line: weak -> strong
    wq, sq = zs["weak_quality"], zs["strong_quality"]
    wc, sc = zs["weak_cost"], zs["strong_cost"]
    plt.plot([wc, sc], [wq, sq], "--", color=COL["random"], lw=1.5, label="random")
    for name in ("panel", "router-bert", "mf"):
        if name in zs["curves"]:
            c = zs["curves"][name]
            plt.plot(c["cost"], c["quality"], color=COL[name], lw=1.8,
                     label=f"{name} (zero-shot, APGR={zs['routers'][name]['APGR']:.2f})")
    if rt:
        c = rt["curves"]["router-rb"]
        plt.plot(c["cost"], c["quality"], color=COL["router-rb"], lw=1.6, ls=":",
                 label=f"bert-mini from scratch (APGR={rt['router-rb']['APGR']:.2f})")
    if emb:
        c = emb["curves"]["router-emb"]
        plt.plot(c["cost"], c["quality"], color="#16A34A", lw=2.4, alpha=0.85,
                 label=f"bge-m3 head, retrained (APGR={emb['router-emb']['APGR']:.2f})")
    plt.scatter([wc, sc], [wq, sq], color=["#888", "#222"], zorder=6)
    plt.annotate("weak (mixtral)", (wc, wq), textcoords="offset points", xytext=(8, -4), fontsize=8)
    plt.annotate("strong (gpt-4)", (sc, sq), textcoords="offset points", xytext=(-90, 4), fontsize=8)
    plt.xlabel("avg cost per query (USD)"); plt.ylabel("avg quality (accuracy)")
    plt.title("RouterBench cost-quality frontier")
    plt.legend(fontsize=8, loc="lower right"); plt.grid(alpha=0.25)
    plt.tight_layout(); plt.savefig(os.path.join(FIG, "cost_quality.png"), dpi=150)
    print("wrote figures/cost_quality.png")


def heat(ax, cm, labels, title):
    M = np.array([[cm[a][b] for b in labels] for a in labels], dtype=float)
    Mn = M / M.sum(axis=1, keepdims=True).clip(min=1)
    ax.imshow(Mn, cmap="Purples", vmin=0, vmax=1)
    ax.set_xticks(range(len(labels))); ax.set_xticklabels(labels, rotation=45, ha="right", fontsize=8)
    ax.set_yticks(range(len(labels))); ax.set_yticklabels(labels, fontsize=8)
    for i in range(len(labels)):
        for j in range(len(labels)):
            ax.text(j, i, int(M[i][j]), ha="center", va="center", fontsize=7,
                    color="white" if Mn[i][j] > 0.5 else "#333")
    ax.set_xlabel("predicted"); ax.set_ylabel("true"); ax.set_title(title, fontsize=9)


def intrinsic():
    d = json.load(open(os.path.join(RES, "intrinsic.json")))
    fig, axes = plt.subplots(1, 2, figsize=(10, 4.2))
    heat(axes[0], d["task_type"]["confusion"], ["qa", "code", "math"],
         f"task type (acc {d['task_type']['accuracy_coarse']})")
    heat(axes[1], d["difficulty_panel"]["confusion"], ["easy", "medium", "hard"],
         f"difficulty panel (acc {d['difficulty_panel']['accuracy']})")
    plt.suptitle("Intrinsic out-of-distribution confusion (synthetic-trained, real prompts)")
    plt.tight_layout(); plt.savefig(os.path.join(FIG, "intrinsic_confusion.png"), dpi=150)
    print("wrote figures/intrinsic_confusion.png")


if __name__ == "__main__":
    os.makedirs(FIG, exist_ok=True)
    intrinsic()
    cost_quality()
