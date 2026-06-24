"""Trivial heuristic routers (the sanity-check baselines).

Each returns a per-prompt route-to-strong propensity in [0,1] so it plugs into the same
cost-quality curve machinery as the ML routers. If a 5-line rule lands near the ML head,
the ML router may not be worth maintaining.
"""
import re

import numpy as np

# Hard-skewing and easy-skewing keyword cues.
HARD_KW = re.compile(
    r"\b(prove|proof|derive|design|architect|optimi[sz]e|complexity|theorem|"
    r"refactor|consensus|distributed|concurren|deadlock|formal|np-hard|integral|"
    r"algorithm|trade-?off|invariant|asymptotic)\b", re.I)
EASY_KW = re.compile(
    r"\b(hi|hello|hey|thanks|thank you|what is|capital of|spell|plural|synonym|"
    r"how are you|good (morning|evening))\b", re.I)
CODE_KW = re.compile(r"```|\bdef \b|\bfn \b|\bfunction \b|\bclass \b|import |SELECT ", re.I)


def length_scores(prompts, cap_chars=1200.0):
    """Longer prompt -> higher escalation propensity."""
    return np.array([min(len(p) / cap_chars, 1.0) for p in prompts], dtype=float)


def keyword_scores(prompts):
    """Rule: hard cues push toward strong, easy cues toward weak; code is mid-high."""
    out = []
    for p in prompts:
        h = len(HARD_KW.findall(p))
        e = len(EASY_KW.findall(p))
        c = 1 if CODE_KW.search(p) else 0
        s = 0.5 + 0.2 * h - 0.3 * e + 0.15 * c
        out.append(min(max(s, 0.0), 1.0))
    return np.array(out, dtype=float)
