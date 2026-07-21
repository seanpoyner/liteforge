#!/usr/bin/env python3
"""Build a real cost-quality dataset for the shipped Claude pool (haiku/sonnet/opus).

For each prompt from graded benchmarks (GSM8K, MMLU, ARC, MBPP auto-graded; MT-Bench via
an LLM judge), run all three Claude tiers through the WG-direct LiteLLM gateway, grade
correctness, and record real cost (LiteLLM's x-litellm-response-cost header, else a price
table). Output a RouterBench-shaped parquet so the existing metric code reuses unchanged.

Cached + resumable: every (tier, prompt) response is cached, so grading/re-runs are free.

    python scripts/eval/build_claude_pool.py --n-gsm8k 200 --n-mmlu 250 --n-arc 150 \
        --n-mbpp 150 --mt-bench --out data/claude_pool.parquet
"""
import argparse
import hashlib
import json
import os
import re
import resource
import subprocess
import sys
import tempfile
import time
import urllib.request

import pandas as pd

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")
CACHE = os.path.join(DATA, "claude_pool_cache")
BASE = os.environ.get("ROUTER_EVAL_BASE_URL", "http://10.8.0.6:4000/v1")  # WG-direct
KEY = os.environ.get("LITEFORGE_API_KEY", "")

TIERS = {"haiku": "claude-haiku-4.5", "sonnet": "claude-sonnet-4.6", "opus": "claude-opus-4.7"}
JUDGE = "claude-opus-4.7"
# Per-1M-token list prices (USD, in/out); fallback only - real cost comes from the
# x-litellm-response-cost header when present.
PRICES = {
    "claude-haiku-4.5": (1.0, 5.0),
    "claude-sonnet-4.6": (3.0, 15.0),
    "claude-opus-4.7": (15.0, 75.0),
}


def _key(tier, pid):
    return hashlib.sha1(f"{tier}|{pid}".encode()).hexdigest()


def call_model(model, messages, max_tokens, cache_id):
    """Call the gateway with caching. Returns dict {text, prompt_tokens, completion_tokens, cost}."""
    os.makedirs(CACHE, exist_ok=True)
    cf = os.path.join(CACHE, cache_id + ".json")
    if os.path.exists(cf):
        return json.load(open(cf))
    body = json.dumps({"model": model, "messages": messages, "max_tokens": max_tokens,
                       "temperature": 0.0}).encode()
    req = urllib.request.Request(BASE.rstrip("/") + "/chat/completions", data=body,
                                 headers={"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"})
    for attempt in range(7):
        try:
            with urllib.request.urlopen(req, timeout=180) as r:
                cost_hdr = r.headers.get("x-litellm-response-cost")
                d = json.load(r)
            break
        except Exception:
            if attempt == 6:
                raise
            time.sleep(min(2 ** attempt, 20))
    text = d["choices"][0]["message"]["content"] or ""
    usage = d.get("usage", {})
    pt, ct = usage.get("prompt_tokens", 0), usage.get("completion_tokens", 0)
    if cost_hdr:
        cost = float(cost_hdr)
    else:
        pin, pout = PRICES.get(model, (0.0, 0.0))
        cost = pt / 1e6 * pin + ct / 1e6 * pout
    out = {"text": text, "prompt_tokens": pt, "completion_tokens": ct, "cost": cost}
    json.dump(out, open(cf, "w"))
    return out


# ---------------------------------------------------------------- dataset loaders
def load_gsm8k(n):
    from datasets import load_dataset
    ds = load_dataset("openai/gsm8k", "main", split="test").shuffle(seed=13).select(range(n))
    rows = []
    for i, r in enumerate(ds):
        gold = r["answer"].split("####")[-1].strip().replace(",", "")
        rows.append({"id": f"gsm8k-{i}", "eval_name": "gsm8k", "grade": "numeric", "gold": gold,
                     "prompt": r["question"] + "\n\nSolve step by step. On the final line write exactly "
                               "'Answer: <number>' and nothing else."})
    return rows


def load_mmlu(n):
    from datasets import load_dataset
    ds = load_dataset("cais/mmlu", "all", split="test").shuffle(seed=13).select(range(n))
    letters = "ABCD"
    rows = []
    for i, r in enumerate(ds):
        ch = "\n".join(f"{letters[j]}. {c}" for j, c in enumerate(r["choices"]))
        rows.append({"id": f"mmlu-{i}", "eval_name": "mmlu", "grade": "mc", "gold": letters[r["answer"]],
                     "prompt": f"{r['question']}\n{ch}\n\nOn the final line write exactly 'Answer: <letter>'."})
    return rows


def load_arc(n):
    from datasets import load_dataset
    ds = load_dataset("allenai/ai2_arc", "ARC-Challenge", split="test").shuffle(seed=13).select(range(n))
    rows = []
    for i, r in enumerate(ds):
        labels = r["choices"]["label"]
        ch = "\n".join(f"{l}. {t}" for l, t in zip(labels, r["choices"]["text"]))
        rows.append({"id": f"arc-{i}", "eval_name": "arc", "grade": "mc", "gold": r["answerKey"],
                     "prompt": f"{r['question']}\n{ch}\n\nOn the final line write exactly 'Answer: <letter>'."})
    return rows


def load_mbpp(n):
    from datasets import load_dataset
    ds = load_dataset("google-research-datasets/mbpp", "sanitized", split="test").shuffle(seed=13).select(range(n))
    rows = []
    for i, r in enumerate(ds):
        # The tests call a specific function name; tell the model to use it exactly.
        m = re.search(r"assert\s+(\w+)\s*\(", r["test_list"][0])
        entry = m.group(1) if m else None
        instr = f"Define a function named `{entry}`. " if entry else ""
        rows.append({"id": f"mbpp-{i}", "eval_name": "mbpp", "grade": "code",
                     "tests": r["test_list"], "setup": r.get("test_setup_code", ""),
                     "prompt": f"{r['prompt']}\n\n{instr}Write a self-contained Python solution. "
                               f"Return only a fenced ```python code block."})
    return rows


def load_mtbench(n):
    from datasets import load_dataset
    for ds_id, field in [("HuggingFaceH4/mt_bench_prompts", "prompt"),
                         ("philschmid/mt-bench", "turns")]:
        try:
            ds = load_dataset(ds_id, split="train")
            rows = []
            for i, r in enumerate(ds):
                q = r[field][0] if isinstance(r.get(field), list) else r.get(field)
                if not q:
                    continue
                rows.append({"id": f"mtbench-{i}", "eval_name": "mtbench", "grade": "judge", "prompt": q})
            if rows:
                return rows[:n]
        except Exception as e:
            print(f"  mt-bench loader {ds_id} failed: {e}")
    print("  mt-bench unavailable; skipping")
    return []


# ---------------------------------------------------------------- graders
def grade_numeric(text, gold):
    m = re.search(r"Answer:\s*\$?(-?\d[\d,]*\.?\d*)", text, re.I)
    cand = m.group(1) if m else None
    if cand is None:
        nums = re.findall(r"-?\d[\d,]*\.?\d*", text.replace(",", ""))
        cand = nums[-1] if nums else None
    if cand is None:
        return 0
    try:
        return int(abs(float(cand.replace(",", "")) - float(gold)) < 1e-4)
    except ValueError:
        return 0


def grade_mc(text, gold):
    m = re.search(r"Answer:\s*[\(<\[]?\s*([A-E])\b", text, re.I)
    if not m:
        # last standalone capital letter as a fallback
        cands = re.findall(r"\b([A-E])\b", text.strip().upper())
        if not cands:
            return 0
        return int(cands[-1] == gold.upper())
    return int(m.group(1).upper() == gold.upper())


def _limits():
    resource.setrlimit(resource.RLIMIT_CPU, (8, 8))
    resource.setrlimit(resource.RLIMIT_AS, (1 << 30, 1 << 30))


def grade_code(text, tests, setup):
    m = re.search(r"```(?:python)?\s*(.*?)```", text, re.DOTALL)
    code = m.group(1) if m else text
    script = code + "\n" + (setup or "") + "\n" + "\n".join(tests)
    with tempfile.NamedTemporaryFile("w", suffix=".py", delete=False) as f:
        f.write(script)
        path = f.name
    try:
        p = subprocess.run([sys.executable, path], capture_output=True, timeout=12,
                           preexec_fn=_limits, env={"PATH": "/usr/bin:/bin"})
        return int(p.returncode == 0)
    except Exception:
        return 0
    finally:
        os.unlink(path)


def grade_judge(question, answer):
    prompt = (f"[Question]\n{question}\n\n[Assistant Answer]\n{answer}\n\n"
              "Rate the answer's helpfulness and correctness from 1 to 10. "
              "Respond with only 'Rating: <n>'.")
    try:
        r = call_model(JUDGE, [{"role": "user", "content": prompt}], 256,
                       _key("judge", hashlib.sha1((question + answer).encode()).hexdigest()))
    except Exception as e:
        print("  judge call failed, scoring 0:", str(e)[:80])
        return 0, 0.0
    m = re.search(r"(\d+(?:\.\d+)?)", r["text"])
    rating = float(m.group(1)) if m else 0.0
    return int(rating >= 7.0), rating


def grade(row, tier, text):
    g = row["grade"]
    if g == "numeric":
        return grade_numeric(text, row["gold"]), None
    if g == "mc":
        return grade_mc(text, row["gold"]), None
    if g == "code":
        return grade_code(text, row["tests"], row.get("setup", "")), None
    if g == "judge":
        return grade_judge(row["prompt"], text)
    return 0, None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n-gsm8k", type=int, default=200)
    ap.add_argument("--n-mmlu", type=int, default=250)
    ap.add_argument("--n-arc", type=int, default=150)
    ap.add_argument("--n-mbpp", type=int, default=150)
    ap.add_argument("--mt-bench", action="store_true")
    ap.add_argument("--n-mtbench", type=int, default=80)
    ap.add_argument("--out", default=os.path.join(DATA, "claude_pool.parquet"))
    args = ap.parse_args()
    if not KEY:
        sys.exit("set LITEFORGE_API_KEY")

    prompts = []
    if args.n_gsm8k: prompts += load_gsm8k(args.n_gsm8k)
    if args.n_mmlu: prompts += load_mmlu(args.n_mmlu)
    if args.n_arc: prompts += load_arc(args.n_arc)
    if args.n_mbpp: prompts += load_mbpp(args.n_mbpp)
    if args.mt_bench: prompts += load_mtbench(args.n_mtbench)
    print(f"loaded {len(prompts)} prompts: "
          + str(pd.Series([p['eval_name'] for p in prompts]).value_counts().to_dict()))

    # Per-benchmark answer-token budgets: stronger models are verbose and were being
    # truncated mid-reasoning before the final answer (which unfairly penalized them).
    answer_tokens = {"gsm8k": 1536, "mmlu": 2048, "arc": 1024, "mbpp": 1536, "mtbench": 2560}
    from concurrent.futures import ThreadPoolExecutor

    # Phase 1: fetch all (prompt, tier) responses concurrently (cached, resumable).
    tasks = [(row, tier, model) for row in prompts for tier, model in TIERS.items()]

    def fetch(t):
        row, tier, model = t
        call_model(model, [{"role": "user", "content": row["prompt"]}],
                   answer_tokens.get(row["eval_name"], 1024), _key(tier, row["id"]))
    done = 0
    with ThreadPoolExecutor(max_workers=12) as ex:
        for _ in ex.map(fetch, tasks):
            done += 1
            if done % 100 == 0:
                print(f"  fetched {done}/{len(tasks)}")
    print("  all responses fetched; grading...")

    # Phase 2: grade concurrently (judge calls + mbpp subprocesses parallelize).
    def grade_row(row):
        rec = {"sample_id": row["id"], "eval_name": row["eval_name"], "prompt": row["prompt"]}
        for tier, model in TIERS.items():
            r = call_model(model, [{"role": "user", "content": row["prompt"]}],
                           answer_tokens.get(row["eval_name"], 1024), _key(tier, row["id"]))
            correct, extra = grade(row, tier, r["text"])
            rec[tier] = float(correct)
            rec[f"{tier}|total_cost"] = r["cost"]
            if extra is not None:
                rec[f"{tier}|rating"] = extra
        order = ["haiku", "sonnet", "opus"]
        rec["oracle"] = next((t for t in order if rec[t] >= 0.5), "none")
        return rec
    with ThreadPoolExecutor(max_workers=8) as ex:
        records = list(ex.map(grade_row, prompts))

    df = pd.DataFrame(records)
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    df.to_parquet(args.out)
    print(f"\nwrote {args.out}  ({len(df)} rows)")
    for t in TIERS:
        print(f"  {t:7s} acc={df[t].mean():.3f}  avg_cost=${df[f'{t}|total_cost'].mean():.5f}")
    print("oracle tier dist:", df["oracle"].value_counts().to_dict())


if __name__ == "__main__":
    main()
