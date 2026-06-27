#!/usr/bin/env python3
"""Render forecast figures from results/forecast.json.

  results/figures/savings_curve.png   - cumulative savings by lever, 3 scenarios
  results/figures/cache_sweep.png     - saved% vs cache hit-rate
  results/figures/tornado.png         - baseline $/mo sensitivity (+/-30%)
  results/figures/build_vs_buy.png    - $/1M: Gemini Flash vs self-host options
"""
import json
import os

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "results")
FIG = os.path.join(OUT, "figures")
COL = {"conservative": "#6272A4", "expected": "#50FA7B", "aggressive": "#FF79C6"}


def _usd(x):
    for u, d in [("B", 1e9), ("M", 1e6), ("K", 1e3)]:
        if abs(x) >= d:
            return f"${x/d:.1f}{u}"
    return f"${x:.0f}"


def savings_curve(d):
    stages = ["cache", "+prune", "+model", "+routed"]
    fig, ax = plt.subplots(figsize=(8, 5))
    x = np.arange(len(stages))
    for i, (name, col) in enumerate(COL.items()):
        sp = d["scenarios"][name]["saved_pct"]
        ys = [sp["cache"], sp["+prune"], sp["+model"], sp["+routed"]]
        ax.plot(x, ys, "-o", color=col, lw=2, label=name)
        for xi, yi in zip(x, ys):
            ax.annotate(f"{yi:.0f}%", (xi, yi), fontsize=7, xytext=(0, 5),
                        textcoords="offset points", ha="center")
    ax.set_xticks(x); ax.set_xticklabels(["+ caching", "+ pruning", "+ model/route", "+ routed"])
    ax.set_ylabel("cumulative cost saved (% of baseline)")
    base = d["baseline"]["usd_mo"]
    ax.set_title(f"Conversational-search savings curve (baseline {_usd(base)}/mo)")
    ax.legend(); ax.grid(alpha=0.3)
    fig.tight_layout(); fig.savefig(os.path.join(FIG, "savings_curve.png"), dpi=150)
    print("wrote savings_curve.png")


def cache_sweep(d):
    rows = d["cache_sweep"]
    fig, ax = plt.subplots(figsize=(7, 4.5))
    hits = [r["hit"] * 100 for r in rows]; saved = [r["saved_pct"] for r in rows]
    ax.plot(hits, saved, "-o", color="#8BE9FD", lw=2)
    for h, s in zip(hits, saved):
        ax.annotate(f"{s:.0f}%", (h, s), fontsize=8, xytext=(0, 5), textcoords="offset points", ha="center")
    ax.set_xlabel("cache hit rate (%)"); ax.set_ylabel("cost saved (% of baseline)")
    ax.set_title("Caching alone: savings vs hit rate (75% prefix coverage)")
    ax.grid(alpha=0.3)
    fig.tight_layout(); fig.savefig(os.path.join(FIG, "cache_sweep.png"), dpi=150)
    print("wrote cache_sweep.png")


def tornado(d):
    rows = d["tornado"][::-1]  # smallest at bottom
    fig, ax = plt.subplots(figsize=(7.5, 4.5))
    base = d["baseline"]["usd_mo"]
    y = np.arange(len(rows))
    for i, r in enumerate(rows):
        ax.barh(i, r["high_+30pct"] - base, left=base, color="#FFB86C", alpha=0.8)
        ax.barh(i, r["low_-30pct"] - base, left=base, color="#BD93F9", alpha=0.8)
    ax.axvline(base, color="#FF5555", lw=1)
    ax.set_yticks(y); ax.set_yticklabels([r["input"] for r in rows])
    ax.set_xlabel("baseline $/mo under +/-30% (red = nominal)")
    ax.set_title("Sensitivity of baseline cost to assumptions")
    fig.tight_layout(); fig.savefig(os.path.join(FIG, "tornado.png"), dpi=150)
    print("wrote tornado.png")


def build_vs_buy(d):
    opts = d.get("model_options", [])
    if not opts:
        return
    kind_col = {"managed": "#50FA7B", "selfhost": "#FF5555"}
    names, vals, cols = [], [], []
    for o in opts:
        short = o["model"].replace(" (Vertex, managed)", "\n(Vertex managed)") \
                          .replace(" (Vertex, managed, est)", "\n(Vertex, est)") \
                          .replace(" (current, Vertex)", "\n(current)") \
                          .replace(" (Amazon self-host est)", "\n(Amazon self-host)")
        names.append(short); vals.append(o["blended_per_1m"])
        cols.append("#8BE9FD" if "current" in o["model"] else kind_col.get(o["kind"], "#BD93F9"))
    fig, ax = plt.subplots(figsize=(9, 4.8))
    ax.bar(range(len(vals)), vals, color=cols, edgecolor="#282A36")
    for i, v in enumerate(vals):
        ax.annotate(f"${v:.2f}", (i, v), fontsize=9, xytext=(0, 4), textcoords="offset points", ha="center")
    ax.set_xticks(range(len(names))); ax.set_xticklabels(names, fontsize=7.5)
    ax.set_ylabel("$ / 1M tokens (blended at conv-search mix)")
    ax.set_title("Model options: Gemma-on-Vertex (managed) undercuts Flash-Lite; Amazon self-host does not")
    fig.tight_layout(); fig.savefig(os.path.join(FIG, "build_vs_buy.png"), dpi=150)
    print("wrote build_vs_buy.png")


def main():
    os.makedirs(FIG, exist_ok=True)
    d = json.load(open(os.path.join(OUT, "forecast.json")))
    savings_curve(d); cache_sweep(d); tornado(d); build_vs_buy(d)


if __name__ == "__main__":
    main()
