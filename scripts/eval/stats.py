"""Bootstrap confidence intervals for routing metrics.

A metric (APGR, cost-saved, accuracy, ...) is recomputed over B resamples of the test
set (sampling prompt indices with replacement); we report mean and a percentile CI.
"""
import numpy as np


def fast_apgr(scores, wq, wc, sq, sc):
    """Vectorized APGR (area between the router cost-quality curve and the random line,
    over the area between oracle and random). O(n log n); matches curve_from_scores+apgr.
    Routes the top-k highest-score prompts to strong, sweeping k = 0..n."""
    wq = np.asarray(wq, float); wc = np.asarray(wc, float)
    sq = np.asarray(sq, float); sc = np.asarray(sc, float)
    n = len(scores)
    tot_wc, tot_wq = wc.sum(), wq.sum()

    def curve(order):
        cum_sc = np.cumsum(sc[order]); cum_wc = np.cumsum(wc[order])
        cum_sq = np.cumsum(sq[order]); cum_wq = np.cumsum(wq[order])
        cost = np.empty(n + 1); qual = np.empty(n + 1)
        cost[0], qual[0] = tot_wc, tot_wq
        cost[1:] = cum_sc + (tot_wc - cum_wc)
        qual[1:] = cum_sq + (tot_wq - cum_wq)
        return cost / n, qual / n

    c, q = curve(np.argsort(-scores))
    oc, oq = curve(np.argsort(-(sq - wq)))
    wqm, sqm, wcm, scm = wq.mean(), sq.mean(), wc.mean(), sc.mean()
    denom = (scm - wcm) if abs(scm - wcm) > 1e-12 else 1.0

    def gain(cc, qq):
        o = np.argsort(cc); cc, qq = cc[o], qq[o]
        randq = wqm + (sqm - wqm) * (cc - wcm) / denom
        return np.trapz(qq - randq, cc)

    go = gain(oc, oq)
    return float(gain(c, q) / go) if abs(go) > 1e-12 else 0.0


def bootstrap_ci(metric_fn, n, b=1000, seed=13, alpha=0.05):
    """metric_fn(idx: np.ndarray) -> float, evaluated over resampled prompt indices.

    Returns (mean, lo, hi) for the (1-alpha) CI.
    """
    rng = np.random.RandomState(seed)
    vals = np.empty(b)
    for i in range(b):
        idx = rng.randint(0, n, n)
        vals[i] = metric_fn(idx)
    vals.sort()
    lo = vals[int(alpha / 2 * b)]
    hi = vals[int((1 - alpha / 2) * b)]
    return float(vals.mean()), float(lo), float(hi)


def fmt_ci(mean, lo, hi, pct=False):
    s = 100 if pct else 1
    u = "%" if pct else ""
    return f"{mean*s:.1f}{u} [{lo*s:.1f}, {hi*s:.1f}]" if pct else f"{mean:.3f} [{lo:.3f}, {hi:.3f}]"
