#!/usr/bin/env python3
"""Cascade ("escalate-on-failure") routing eval - the easier alternative to up-front difficulty
prediction. Predictive routers score ~random on agentic tau (see routing_eval); here we instead
run the cheap tier first and escalate only when it fails, comparing four escalation triggers on
the cost-quality plane against the predictive routers and single-model baselines.

    python scripts/eval/agentic/cascade_eval.py --pool ollama --env airline --n 50 --proto v5
    python scripts/eval/agentic/cascade_eval.py --judge-model granite4.1:8b   # cheaper verifier

Triggers (escalate=True -> go to next tier):
  oracle-cascade  : escalate iff that tier actually failed (reward<0.5)         [ceiling]
  give-up-signal  : escalate iff finish==max_steps or transfer_to_human fired   [naive]
  self-consistency: escalate iff the cheap tier's trials disagree (rate not 0/1) [free]
  judge-cascade   : a small judge reads policy+transcript, escalate iff "not completed" [proposed]
"""
import argparse
import json
import os
import re
import sys

import numpy as np

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, _HERE)
sys.path.insert(0, os.path.join(_HERE, ".."))
from run_agent import POOLS, CACHE, PROTO, OLLAMA_PRICES, call_chat, _key  # noqa: E402
from stats import bootstrap_ci, fmt_ci  # noqa: E402

ORDER = ["weak", "medium", "strong"]
DATA = os.path.dirname(CACHE)


def _proxy_cost(model, pt, ct):
    pin, pout = OLLAMA_PRICES.get(model, (0.0, 0.0))
    return pt / 1e6 * pin + ct / 1e6 * pout


def _transfer(tr):
    for m in tr:
        if m.get("role") == "assistant":
            tcs = m.get("tool_calls") or []
            if tcs and (tcs[0].get("function") or {}).get("name") == "transfer_to_human_agents":
                return True
    return False


def load_episode(bench, env, task, model, trial, proto):
    cf = os.path.join(CACHE, _key(bench, proto, env, task, model, trial) + ".json")
    if not os.path.exists(cf):
        return None
    d = json.load(open(cf))
    tr = d.get("transcript") or []
    return {"success": float(d.get("success", 0)), "reward": float(d.get("reward", 0)),
            "finish": d.get("finish"), "transfer": _transfer(tr),
            "cost": _proxy_cost(model, d.get("prompt_tokens", 0), d.get("completion_tokens", 0)),
            "transcript": tr}


def load_grid(pool, bench, env, n, proto, max_trials=8):
    tiers = POOLS[pool]
    grid = {}; tasks = []
    for t in range(n):
        cell = {}; ok = True
        for tier, model in tiers.items():
            eps = []
            for k in range(max_trials):
                e = load_episode(bench, env, t, model, k, proto)
                if e is None:
                    break
                eps.append(e)
            if not eps:
                ok = False; break
            cell[tier] = eps
        if ok:
            grid[t] = cell; tasks.append(t)
    return tiers, tasks, grid


# -------------------------------------------------------------------- cascade sim
def simulate(tasks, grid, trigger):
    """trigger(tier, ep, cell, k) -> escalate? Returns per-task (quality, cost) arrays."""
    Q = []; Cst = []
    for t in tasks:
        cell = grid[t]
        K = min(len(cell[tier]) for tier in ORDER)
        qs = []; cs = []
        for k in range(K):
            cost = 0.0; quality = 0.0
            for ti, tier in enumerate(ORDER):
                ep = cell[tier][k]
                cost += ep["cost"]
                if ti == len(ORDER) - 1 or not trigger(tier, ep, cell, k):
                    quality = ep["success"]; break
            qs.append(quality); cs.append(cost)
        Q.append(float(np.mean(qs))); Cst.append(float(np.mean(cs)))
    return np.array(Q), np.array(Cst)


def single_tier(tasks, grid, tier):
    Q = [float(np.mean([e["success"] for e in grid[t][tier]])) for t in tasks]
    C = [float(np.mean([e["cost"] for e in grid[t][tier]])) for t in tasks]
    return np.array(Q), np.array(C)


# ---------------------------------------------------------------------- judge
def _wiki(env):
    try:
        if env == "airline":
            from tau_bench.envs.airline.wiki import WIKI
        else:
            from tau_bench.envs.retail.wiki import WIKI
        return WIKI
    except Exception:
        return ""


def _render(tr, maxlen=4500):
    out = []
    for m in tr:
        role = m.get("role")
        if role == "assistant":
            tcs = m.get("tool_calls") or []
            if tcs:
                fn = tcs[0].get("function", {})
                out.append(f"AGENT->tool {fn.get('name')}({str(fn.get('arguments',''))[:300]})")
            elif m.get("content"):
                out.append(f"AGENT: {str(m['content'])[:300]}")
        elif role == "tool":
            out.append(f"TOOL[{m.get('name')}]: {str(m.get('content'))[:200]}")
        elif role == "user":
            out.append(f"USER: {str(m.get('content'))[:300]}")
    return "\n".join(out)[:maxlen]


def judge_decisions(tasks, grid, env, judge_model, proto, pool, workers=8):
    from concurrent.futures import ThreadPoolExecutor
    safe = judge_model.replace("/", "_").replace(":", "_")
    cf = os.path.join(DATA, f"judge_{safe}_{pool}_{env}_{proto}.json")
    cache = json.load(open(cf)) if os.path.exists(cf) else {}
    sysmsg = ("You are a QA verifier for a customer-service agent. Read the POLICY and the "
              "conversation, then decide whether the agent completed the user's request. Answer NO "
              "ONLY if there is a clear, concrete problem: a policy violation, a wrong or missing "
              "action, or a stated user requirement that was not met. If the request appears "
              "reasonably satisfied, answer YES. Most competent completions should be YES; reserve "
              "NO for genuine failures. Reason briefly, then end with a final line exactly "
              "'VERDICT: YES' or 'VERDICT: NO'.\n\nPOLICY:\n" + _wiki(env))

    def judge_one(item):
        key, ep = item
        tr = ep["transcript"]
        opening = next((str(m.get("content")) for m in tr if m.get("role") == "user"), "")
        user = (f"USER REQUEST (opening):\n{opening}\n\nCONVERSATION:\n{_render(tr)}\n\n"
                "Did the agent complete the request per policy? Reason briefly, then end "
                "with 'VERDICT: YES' or 'VERDICT: NO'.")
        try:
            msg, _, _ = call_chat(judge_model, [{"role": "system", "content": sysmsg},
                                  {"role": "user", "content": user}], tools=None,
                                  max_tokens=2048, temperature=0.0)
            mv = re.search(r"VERDICT:\s*(YES|NO)", (msg.get("content") or "").upper())
            completed = (mv.group(1) == "YES") if mv else True  # fail-open if no verdict line
        except Exception:
            completed = True  # fail-open: judge error -> don't escalate
        return key, completed

    todo = [(f"{t}|{tier}|{k}", ep) for t in tasks for tier in ("weak", "medium")
            for k, ep in enumerate(grid[t][tier]) if f"{t}|{tier}|{k}" not in cache]
    if todo:
        print(f"  judging {len(todo)} uncached transcripts with {workers} workers ...", flush=True)
        done = 0
        with ThreadPoolExecutor(max_workers=workers) as ex:
            for key, completed in ex.map(judge_one, todo):
                cache[key] = {"completed": completed}
                done += 1
                if done % 20 == 0:
                    json.dump(cache, open(cf, "w")); print(f"    {done}/{len(todo)}", flush=True)
        json.dump(cache, open(cf, "w"))

    st = {"tp": 0, "fp": 0, "tn": 0, "fn": 0}
    for t in tasks:
        for tier in ("weak", "medium"):
            for k, ep in enumerate(grid[t][tier]):
                esc = not cache[f"{t}|{tier}|{k}"]["completed"]
                fail = ep["success"] < 0.5
                st["tp" if (esc and fail) else "fp" if esc else "fn" if fail else "tn"] += 1
    return cache, st


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pool", default="ollama", choices=list(POOLS))
    ap.add_argument("--bench", default="tau")
    ap.add_argument("--env", default="airline")
    ap.add_argument("--n", type=int, default=50)
    ap.add_argument("--proto", default=PROTO)
    ap.add_argument("--judge-model", default="qwen3:14b")
    ap.add_argument("--no-judge", action="store_true")
    args = ap.parse_args()

    tiers, tasks, grid = load_grid(args.pool, args.bench, args.env, args.n, args.proto)
    if len(tasks) < 3:
        print(f"only {len(tasks)} tasks complete for pool={args.pool}; run the bench further."); return
    n = len(tasks)
    print(f"pool={args.pool} {args.bench}/{args.env}  n={n} tasks  proto={args.proto}\n")

    rng = np.random.RandomState(13)
    rows = []  # (name, Qarr, Carr)
    # single-model baselines
    for tier in ORDER:
        rows.append((f"all-{tier}", *single_tier(tasks, grid, tier)))
    # random tier (per task pick one tier's trial-mean)
    rpick = rng.randint(0, 3, n)
    rQ = np.array([single_tier([tasks[i]], grid, ORDER[rpick[i]])[0][0] for i in range(n)])
    rC = np.array([single_tier([tasks[i]], grid, ORDER[rpick[i]])[1][0] for i in range(n)])
    rows.append(("random-tier", rQ, rC))

    # cascades
    rows.append(("cascade:oracle", *simulate(tasks, grid, lambda tier, ep, c, k: ep["reward"] < 0.5)))
    rows.append(("cascade:give-up", *simulate(tasks, grid,
                 lambda tier, ep, c, k: ep["finish"] == "max_steps" or ep["transfer"])))
    rows.append(("cascade:self-consist", *simulate(tasks, grid,
                 lambda tier, ep, c, k: 0 < np.mean([e["success"] for e in c[tier]]) < 1)))

    judge_st = None
    if not args.no_judge:
        print(f"running judge={args.judge_model} over weak+medium transcripts ...", flush=True)
        jcache, judge_st = judge_decisions(tasks, grid, args.env, args.judge_model, args.proto, args.pool)
        rows.append((f"cascade:judge({args.judge_model})", *simulate_judge(tasks, grid, jcache)))

    # predictive routers (3-tier decision points) from routing_eval output, if present
    rj = os.path.join(DATA, f"agentic_routing_{args.pool}_{args.env}.json")
    if os.path.exists(rj):
        rr = json.load(open(rj)).get("routers", {})
        for name in ("router-bert", "mf"):
            if name in rr and "point_q" in rr[name]:
                rows.append((f"predict:{name}", np.full(n, rr[name]["point_q"]),
                             np.full(n, rr[name]["point_c"])))

    # report
    strong_q = single_tier(tasks, grid, "strong")[0].mean()
    strong_c = single_tier(tasks, grid, "strong")[1].mean()
    print(f"\n{'strategy':26s} {'quality (95% CI)':24s} {'$/task':>9s} {'%strongQ':>9s} {'%strongC':>9s}")
    out = {}
    for name, Q, C in rows:
        m, lo, hi = bootstrap_ci(lambda ix, a=Q: a[ix].mean(), n, b=2000)
        cc = C.mean()
        print(f"{name:26s} {fmt_ci(m,lo,hi):24s} {cc:9.4f} "
              f"{100*m/strong_q if strong_q else 0:8.1f}% {100*cc/strong_c if strong_c else 0:8.1f}%")
        out[name] = {"quality": float(m), "q_lo": float(lo), "q_hi": float(hi), "cost": float(cc)}

    if judge_st:
        tp, fp, fn, tn = judge_st["tp"], judge_st["fp"], judge_st["fn"], judge_st["tn"]
        prec = tp / (tp + fp) if tp + fp else 0
        rec = tp / (tp + fn) if tp + fn else 0
        print(f"\njudge={args.judge_model} as failure-detector: precision={prec:.2f} recall={rec:.2f} "
              f"(tp={tp} fp={fp} fn={fn} tn={tn})")
        out["_judge"] = {"model": args.judge_model, "precision": prec, "recall": rec, **judge_st}

    json.dump({"pool": args.pool, "env": args.env, "n": n, "proto": args.proto,
               "strong_q": float(strong_q), "strong_c": float(strong_c), "strategies": out},
              open(os.path.join(DATA, f"agentic_cascade_{args.pool}_{args.env}.json"), "w"), indent=2)


def simulate_judge(tasks, grid, jcache):
    """Cascade where escalate = judge said 'not completed' for that (task,tier,trial)."""
    def trig_for(t):
        def trig(tier, ep, c, k):
            ent = jcache.get(f"{t}|{tier}|{k}")
            return (not ent["completed"]) if ent else False
        return trig
    Q = []; Cst = []
    for t in tasks:
        cell = grid[t]; trig = trig_for(t)
        K = min(len(cell[tier]) for tier in ORDER)
        qs = []; cs = []
        for k in range(K):
            cost = 0.0; quality = 0.0
            for ti, tier in enumerate(ORDER):
                ep = cell[tier][k]
                cost += ep["cost"]
                if ti == len(ORDER) - 1 or not trig(tier, ep, cell, k):
                    quality = ep["success"]; break
            qs.append(quality); cs.append(cost)
        Q.append(float(np.mean(qs))); Cst.append(float(np.mean(cs)))
    return np.array(Q), np.array(Cst)


if __name__ == "__main__":
    main()
