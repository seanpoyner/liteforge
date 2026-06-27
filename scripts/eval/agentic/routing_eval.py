#!/usr/bin/env python3
"""Build the task x tier success/cost matrix from cached agentic episodes, then evaluate
routing: oracle / random baselines vs our trained routers (router-bert hardness classifier and
the MF/bge-m3 RouteLLM-port head), with bootstrap CIs, APGR, and 3-tier decision points. Also a
power calc for the minimum N at 80% power.

Cost axis: the ollama tiers are served free via the gateway ($0), so for that pool we use a
param-scaled compute-cost PROXY (run_agent.OLLAMA_PRICES) applied to the measured tokens; other
pools use the gateway cost. Select with --cost-model {param,gateway} (default: param for ollama).

Works on partial caches (only tasks completed for ALL tiers in a pool are used).

    python scripts/eval/agentic/routing_eval.py --pool ollama --env airline --n 50 --proto v5
"""
import argparse
import hashlib
import json
import os
import sys

import numpy as np

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, _HERE)                          # agentic/ (run_agent)
sys.path.insert(0, os.path.join(_HERE, ".."))      # scripts/eval (stats, routerbench_eval, router_runner)
from run_agent import POOLS, CACHE, PROTO, OLLAMA_PRICES  # noqa: E402
from stats import bootstrap_ci, fmt_ci, fast_apgr  # noqa: E402
from routerbench_eval import curve_from_scores, cost_saved_at_quality  # noqa: E402


def _key(*parts):
    return hashlib.sha1("|".join(map(str, parts)).encode()).hexdigest()


def _load_trials(bench, env, i, model, max_trials=16, proto=PROTO):
    """Load all cached trials for one (task, model); return aggregated dict or None.
    Aggregates the per-task success RATE, gateway cost, and mean prompt/completion tokens."""
    trs = []
    for k in range(max_trials):
        cf = os.path.join(CACHE, _key(bench, proto, env, i, model, k) + ".json")
        if not os.path.exists(cf):
            break
        trs.append(json.load(open(cf)))
    if not trs:
        return None
    m = len(trs)
    return {"success": sum(x["success"] for x in trs) / m,        # per-task success RATE
            "cost": sum(x["cost"] for x in trs) / m,              # gateway $ (0 for ollama)
            "pt": sum(x.get("prompt_tokens", 0) for x in trs) / m,
            "ct": sum(x.get("completion_tokens", 0) for x in trs) / m,
            "trials": m}


def load_matrix(pool, bench, env, n, proto=PROTO):
    """Return per-tier arrays of (success-rate, cost, tokens) for tasks completed across ALL tiers."""
    tiers = POOLS[pool]  # {weak,medium,strong: model_id}
    rows = []
    for i in range(n):
        cells = {}
        ok = True
        for tname, model in tiers.items():
            agg = _load_trials(bench, env, i, model, proto=proto)
            if agg is None:
                ok = False; break
            cells[tname] = agg
        if ok:
            rows.append((i, cells))
    return tiers, rows


def matrices(rows, tiers):
    idx = [i for i, _ in rows]
    S = {t: np.array([c[t]["success"] for _, c in rows], float) for t in tiers}
    C = {t: np.array([c[t]["cost"] for _, c in rows], float) for t in tiers}
    PT = {t: np.array([c[t]["pt"] for _, c in rows], float) for t in tiers}
    CT = {t: np.array([c[t]["ct"] for _, c in rows], float) for t in tiers}
    return idx, S, C, PT, CT


def tier_cost(tiers, PT, CT, gatewayC, cost_model):
    """Per-tier per-task cost array. cost_model='param' uses OLLAMA_PRICES x measured tokens for
    any model priced there; otherwise (or 'gateway') falls back to the gateway $ array."""
    out = {}
    for t, model in tiers.items():
        if cost_model == "param" and model in OLLAMA_PRICES:
            pin, pout = OLLAMA_PRICES[model]
            out[t] = PT[t] / 1e6 * pin + CT[t] / 1e6 * pout
        else:
            out[t] = gatewayC[t]
    return out


def oracle_cost_quality(S, C, order):
    """Oracle: cheapest tier (by `order` weak->strong) that succeeds; else strongest.
    Returns (quality, cost) per task picking the cheapest successful tier."""
    n = len(S[order[0]])
    q = np.zeros(n); cost = np.zeros(n)
    for k in range(n):
        chosen = order[-1]
        for t in order:
            if S[t][k] >= 0.5:
                chosen = t; break
        q[k] = S[chosen][k]; cost[k] = C[chosen][k]
    return q, cost


# ---------------------------------------------------------------- router scoring
def _task_text(bench, env, i, model, proto):
    """The decision-time router input: the opening user message from a cached transcript.
    Same across tiers (shared task_index), so we read the weak tier's episode."""
    cf = os.path.join(CACHE, _key(bench, proto, env, i, model, 0) + ".json")
    if not os.path.exists(cf):
        return None
    tr = json.load(open(cf)).get("transcript") or []
    for msg in tr:
        if msg.get("role") == "user":
            return str(msg.get("content") or "")
    return None


def score_routers(idx, texts, pool, env, proto):
    """Return {'router-bert': {'hard': np.array, 'tier': [labels]}, 'mf': {'score': np.array}}.
    Routers are deterministic -> cache per-task scores to disk so re-runs are instant.
    router-bert is local (always tried); MF needs bge-m3 embeddings via the gateway (optional)."""
    cachef = os.path.join(os.path.dirname(CACHE), f"route_scores_{pool}_{env}_{proto}.json")
    cache = json.load(open(cachef)) if os.path.exists(cachef) else {}
    dirty = False
    out = {}

    # router-bert (local inference)
    try:
        from router_runner import BertRunner
        bert = BertRunner()
        hard = []; tier = []
        for i, txt in zip(idx, texts):
            key = str(i)
            ent = cache.get(key, {})
            if "bert_probs" not in ent:
                ent["bert_probs"] = bert.probs(txt or "")
                cache[key] = ent; dirty = True
            p = ent["bert_probs"]
            hard.append(float(p.get("hard", 0.0)))
            tier.append(max(p, key=p.get))  # easy / medium / hard
        out["router-bert"] = {"hard": np.array(hard), "label": tier}
    except Exception as e:
        print(f"  [router-bert unavailable: {str(e)[:80]}]")

    # MF / bge-m3 head (needs embeddings via the gateway)
    try:
        from router_runner import MfRunner
        need = [str(i) for i in idx if "mf" not in cache.get(str(i), {})]
        if need:
            mf = MfRunner()
            need_txt = [texts[idx.index(int(k))] or "" for k in need]
            scores = mf.hardness_batch(need_txt, batch=32)
            for k, sc in zip(need, scores):
                cache.setdefault(k, {})["mf"] = float(sc); dirty = True
        out["mf"] = {"score": np.array([cache[str(i)]["mf"] for i in idx])}
    except Exception as e:
        print(f"  [MF router unavailable (embeddings?): {str(e)[:80]}]")

    if dirty:
        json.dump(cache, open(cachef, "w"))
    return out


def _label_to_tier(label):
    return {"easy": "weak", "medium": "medium", "hard": "strong"}.get(label, "medium")


def decision_point(tier_choice, S, C):
    """Realized (mean quality, mean cost) for a per-task tier assignment."""
    n = len(tier_choice)
    q = np.array([S[tier_choice[k]][k] for k in range(n)])
    c = np.array([C[tier_choice[k]][k] for k in range(n)])
    return float(q.mean()), float(c.mean())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pool", default="ollama", choices=list(POOLS))
    ap.add_argument("--bench", default="tau")
    ap.add_argument("--env", default="airline")
    ap.add_argument("--n", type=int, default=50)
    ap.add_argument("--proto", default=PROTO, help=f"protocol generation to analyze (default {PROTO})")
    ap.add_argument("--cost-model", default=None, choices=["param", "gateway"],
                    help="cost axis (default: param for ollama, gateway otherwise)")
    ap.add_argument("--no-routers", action="store_true", help="skip router scoring (baselines only)")
    args = ap.parse_args()

    cost_model = args.cost_model or ("param" if args.pool.startswith("ollama") else "gateway")
    tiers, rows = load_matrix(args.pool, args.bench, args.env, args.n, proto=args.proto)
    if len(rows) < 3:
        print(f"only {len(rows)} tasks complete across all tiers for pool={args.pool}; "
              f"run the bench further."); return
    order = ["weak", "medium", "strong"]
    idx, S, gatewayC, PT, CT = matrices(rows, tiers)
    C = tier_cost(tiers, PT, CT, gatewayC, cost_model)
    n = len(idx)
    print(f"pool={args.pool} bench={args.bench}/{args.env}  n={n} tasks complete  cost-model={cost_model}")
    print(f"tiers: {tiers}\n")
    print(f"{'tier':8s} {'model':26s} {'success(95% CI)':22s} {'$/task':>9s} {'avg_tok(in/out)':>16s}")
    for t in order:
        m, lo, hi = bootstrap_ci(lambda ix, a=S[t]: a[ix].mean(), n, b=2000)
        print(f"{t:8s} {tiers[t]:26s} {fmt_ci(m,lo,hi):22s} {C[t].mean():9.4f} "
              f"{PT[t].mean():7.0f}/{CT[t].mean():<7.0f}")

    # routable structure (the headroom routing needs)
    w, s = S["weak"], S["strong"]
    both = int(((w >= .5) & (s >= .5)).sum()); wonly = int(((w >= .5) & (s < .5)).sum())
    sonly = int(((w < .5) & (s >= .5)).sum()); neither = int(((w < .5) & (s < .5)).sum())
    print(f"\nweak vs strong: both={both} weak-only={wonly} strong-only={sonly} neither={neither}")
    print(f"  routable (strong succeeds where weak fails) = {sonly}/{n} = {sonly/n:.2f}")

    # cost-quality reference points
    oq, oc = oracle_cost_quality(S, C, order)
    rng = np.random.RandomState(13)
    rand_pick = rng.randint(0, 3, n)
    rq = np.array([S[order[rand_pick[k]]][k] for k in range(n)])
    rc = np.array([C[order[rand_pick[k]]][k] for k in range(n)])
    print(f"\n{'strategy':16s} {'quality':>8s} {'$/task':>9s}")
    print(f"{'all-weak':16s} {w.mean():8.3f} {C['weak'].mean():9.4f}")
    print(f"{'all-medium':16s} {S['medium'].mean():8.3f} {C['medium'].mean():9.4f}")
    print(f"{'all-strong':16s} {s.mean():8.3f} {C['strong'].mean():9.4f}")
    print(f"{'random-tier':16s} {rq.mean():8.3f} {rc.mean():9.4f}")
    print(f"{'oracle':16s} {oq.mean():8.3f} {oc.mean():9.4f}")

    # ---------------- routers: APGR (binary weak-vs-strong, RouteLLM frame) + 3-tier point
    router_out = {}
    if not args.no_routers:
        texts = [_task_text(args.bench, args.env, i, tiers["weak"], args.proto) for i in idx]
        missing = sum(t is None for t in texts)
        if missing:
            print(f"\n[warn: {missing}/{n} task texts missing from transcripts; routing those as empty]")
        scored = score_routers(idx, texts, args.pool, args.env, args.proto)

        wq, wc, sq, sc = S["weak"], C["weak"], S["strong"], C["strong"]
        # oracle/random APGR bookends (sanity: ~1.0 and ~0.0)
        oracle_sc = (sq - wq) + 1e-6 * rng.rand(n)
        rand_sc = rng.rand(n)
        print(f"\n=== routers: binary weak-vs-strong APGR (RouteLLM frame) ===")
        print(f"{'router':14s} {'APGR (95% CI)':24s} {'cost_saved@95%strong':>20s}")

        def apgr_row(name, sc_arr):
            a, lo, hi = bootstrap_ci(
                lambda ix: fast_apgr(sc_arr[ix], wq[ix], wc[ix], sq[ix], sc[ix]), n, b=2000)
            cc, qq = curve_from_scores(sc_arr, wq, wc, sq, sc)
            saved = cost_saved_at_quality(cc, qq, 0.95 * sq.mean(), sc.mean())
            print(f"{name:14s} {fmt_ci(a,lo,hi):24s} {(str(saved)+'%' if saved is not None else 'n/a'):>20s}")
            return {"apgr": a, "apgr_lo": lo, "apgr_hi": hi, "cost_saved_95": saved}

        router_out["oracle"] = apgr_row("oracle", oracle_sc)
        router_out["random"] = apgr_row("random", rand_sc)
        for name in ("router-bert", "mf"):
            if name in scored:
                sc_arr = scored[name]["hard"] if name == "router-bert" else scored[name]["score"]
                router_out[name] = apgr_row(name, sc_arr)

        # 3-tier decision points (each router commits to one tier per task)
        print(f"\n=== routers: 3-tier decision point (realized) ===")
        print(f"{'router':14s} {'quality':>8s} {'$/task':>9s}  mapping")
        if "router-bert" in scored:
            choice = [_label_to_tier(l) for l in scored["router-bert"]["label"]]
            q3, c3 = decision_point(choice, S, C)
            print(f"{'router-bert':14s} {q3:8.3f} {c3:9.4f}  easy->weak/medium->medium/hard->strong")
            router_out.setdefault("router-bert", {}).update({"point_q": q3, "point_c": c3})
        if "mf" in scored:
            mfsc = scored["mf"]["score"]
            t1, t2 = np.quantile(mfsc, [1 / 3, 2 / 3])
            choice = ["weak" if x < t1 else ("medium" if x < t2 else "strong") for x in mfsc]
            q3, c3 = decision_point(choice, S, C)
            print(f"{'mf':14s} {q3:8.3f} {c3:9.4f}  hardness tertiles -> weak/medium/strong")
            router_out.setdefault("mf", {}).update({"point_q": q3, "point_c": c3})

    # power: min N at 80% power to detect the oracle-vs-allweak quality gain
    gap = oq - w
    eff = gap.mean(); sd = gap.std() + 1e-9
    need = ((1.96 + 0.84) * sd / eff) ** 2 if eff > 0 else float("inf")
    print(f"\npower: oracle-vs-allweak quality gain {eff:.3f} (sd {sd:.3f}); "
          f"min N at 80% power ~ {need:.0f} (current n={n}); MDE at n={n}: "
          f"{(1.96+0.84)*sd/np.sqrt(n):.3f}")

    json.dump({"pool": args.pool, "env": args.env, "n": n, "proto": args.proto,
               "cost_model": cost_model, "tiers": tiers,
               "success": {t: float(S[t].mean()) for t in order},
               "cost": {t: float(C[t].mean()) for t in order},
               "routable": sonly, "both": both, "strong_only": sonly, "weak_only": wonly,
               "oracle_q": float(oq.mean()), "oracle_c": float(oc.mean()),
               "all_weak_q": float(w.mean()), "all_strong_q": float(s.mean()),
               "all_strong_c": float(C["strong"].mean()),
               "routers": router_out},
              open(os.path.join(os.path.dirname(CACHE), f"agentic_routing_{args.pool}_{args.env}.json"), "w"),
              indent=2)


if __name__ == "__main__":
    main()
