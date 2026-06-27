#!/usr/bin/env python3
"""Render the cross-provider study figures from results/provider_frontier.json.

  results/figures/provider_frontier.png   - accuracy vs $/1M tokens, Pareto frontier
  results/figures/provider_heatmap.png    - per-benchmark accuracy (where cheap models break)
  results/figures/provider_breakeven.png  - build-vs-buy: monthly cost vs volume

    python scripts/eval/provider_plots.py
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
PATH_COL = {"hosted": "#8BE9FD", "selfhost": "#50FA7B", "anchor": "#FF79C6"}


def _load():
    return json.load(open(os.path.join(RES, "provider_frontier.json")))


def frontier(d):
    fig, ax = plt.subplots(figsize=(8, 5.5))
    pts = [m for m in d["models"] if m["usd_per_1m"]]
    for m in pts:
        ax.scatter(m["usd_per_1m"], m["acc"], s=70, color=PATH_COL.get(m["path"], "#BD93F9"),
                   edgecolor="#282A36", zorder=3)
        ax.annotate(m["model"], (m["usd_per_1m"], m["acc"]), fontsize=7,
                    xytext=(4, 4), textcoords="offset points")
    # Pareto frontier (max acc per increasing cost).
    s = sorted(pts, key=lambda m: m["usd_per_1m"])
    fx, fy, best = [], [], -1
    for m in s:
        if m["acc"] > best:
            best = m["acc"]; fx.append(m["usd_per_1m"]); fy.append(m["acc"])
    ax.step(fx, fy, where="post", color="#FFB86C", lw=1.5, alpha=0.8, label="Pareto frontier")
    ax.axhline(d["sonnet_acc"], color="#FF5555", ls="--", lw=1, label="sonnet quality")
    ax.axhline(0.95 * d["sonnet_acc"], color="#FF5555", ls=":", lw=1, alpha=0.6, label="95% sonnet")
    ax.set_xscale("log")
    ax.set_xlabel("cost  (USD / 1M tokens, own-path price; log scale)")
    ax.set_ylabel("accuracy (public benchmark pool)")
    ax.set_title("Cross-provider cost-quality frontier")
    handles = [plt.Line2D([], [], marker="o", ls="", color=c, label=p)
               for p, c in PATH_COL.items()]
    ax.legend(handles=handles + ax.get_legend_handles_labels()[0], fontsize=8, loc="lower right")
    fig.tight_layout(); fig.savefig(os.path.join(FIG, "provider_frontier.png"), dpi=150)
    print("wrote provider_frontier.png")


def heatmap(d):
    models = [m["model"] for m in d["models"]]
    benches = d["benches"]
    M = np.array([[m["per_bench"][b] for b in benches] for m in d["models"]])
    fig, ax = plt.subplots(figsize=(1.2 + 1.0 * len(benches), 0.5 * len(models) + 1.5))
    im = ax.imshow(M, cmap="RdYlGn", vmin=0, vmax=1, aspect="auto")
    ax.set_xticks(range(len(benches))); ax.set_xticklabels(benches, rotation=40, ha="right", fontsize=8)
    ax.set_yticks(range(len(models))); ax.set_yticklabels(models, fontsize=8)
    for i in range(len(models)):
        for j in range(len(benches)):
            ax.text(j, i, f"{M[i, j]:.2f}", ha="center", va="center", fontsize=7,
                    color="#282A36")
    ax.set_title("Where models break (accuracy by benchmark)")
    fig.colorbar(im, ax=ax, fraction=0.025)
    fig.tight_layout(); fig.savefig(os.path.join(FIG, "provider_heatmap.png"), dpi=150)
    print("wrote provider_heatmap.png")


def breakeven(d):
    be = d.get("breakeven", {})
    cheap = next((m for m in d["models"] if m["path"] == "hosted" and m["usd_per_1m"]), None)
    if not cheap:
        return
    h = cheap["usd_per_1m"]
    vol = np.logspace(6, 11, 200)  # 1M .. 100B tokens/month
    fig, ax = plt.subplots(figsize=(8, 5))
    ax.plot(vol, h * vol / 1e6, color="#8BE9FD", lw=2, label=f"hosted {cheap['model']} (${h:.2f}/1M)")
    for gpu, dd in be.items():
        if not isinstance(dd, dict) or "fixed_monthly_usd" not in dd:
            continue
        ax.plot(vol, np.full_like(vol, dd["fixed_monthly_usd"]), lw=1.5,
                label=f"own {gpu} (${dd['fixed_monthly_usd']:.0f}/mo fixed)")
    ax.axhspan(1500, 5000, color="#FFB86C", alpha=0.15, label="watsonx floor $1.5-5k/mo")
    ax.set_xscale("log"); ax.set_yscale("log")
    ax.set_xlabel("monthly volume (tokens)"); ax.set_ylabel("monthly cost (USD)")
    ax.set_title("Build vs buy: monthly cost vs volume")
    ax.legend(fontsize=8)
    fig.tight_layout(); fig.savefig(os.path.join(FIG, "provider_breakeven.png"), dpi=150)
    print("wrote provider_breakeven.png")


def main():
    os.makedirs(FIG, exist_ok=True)
    d = _load()
    frontier(d); heatmap(d); breakeven(d)


if __name__ == "__main__":
    main()
