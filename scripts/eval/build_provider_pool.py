#!/usr/bin/env python3
"""Build a cross-provider cost-quality dataset spanning four economic paths.

Generalizes build_claude_pool.py from 3 fixed Claude tiers to an arbitrary model list:
  - hosted-API cheap (gemini flash-lite, gpt nano/mini, deepseek-flash, mistral-small)
  - self-host / "buy compute" (granite, qwen, gemma, gpt-oss on hal's GB10)
  - watsonx-class (IBM Granite, measured via the self-hosted granite as a quality proxy)
  - Claude anchor (haiku cheap reference, sonnet quality ceiling)

Benchmarks chosen to SEPARATE the pool (cheap/OSS models genuinely fall below the anchor):
GSM8K (numeric), MMLU + ARC (4-way MC), MBPP (sandboxed code), GPQA-diamond (hard science
MC), MMLU-Pro (10-way MC), and a strict-JSON-to-schema probe (the agent reliability gate).
Long-context is deferred (ollama num_ctx defaults would confound it).

Reuses build_claude_pool's graders, dataset loaders, and the existing claude_pool_cache
(so haiku/sonnet on the shared slices are free). Records correctness + measured hosted cost
+ completion tokens + wall latency + tokens/sec (the last two feed the owned-compute math).

    python scripts/eval/build_provider_pool.py --pilot --out data/provider_pool.parquet
"""
import argparse
import hashlib
import json
import os
import re
import sys
import time
import urllib.request
from concurrent.futures import ThreadPoolExecutor

import pandas as pd

import build_claude_pool as bcp  # graders, dataset loaders, cache machinery

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")
CACHE = bcp.CACHE  # share claude_pool_cache: haiku/sonnet on shared slices reuse it
BASE = os.environ.get("ROUTER_EVAL_BASE_URL", "http://10.8.0.6:4000/v1")
KEY = os.environ.get("LITEFORGE_API_KEY", "")

# alias -> (gateway model id, economic path). Path drives concurrency + the pricing overlay.
MODELS = {
    # hosted-API cheap (opex, zero capex)
    "gemini-flash-lite": ("gemini-3.1-flash-lite", "hosted"),
    "gpt-nano":          ("gpt-5.2-nano",          "hosted"),
    "gpt-mini":          ("gpt-5.2-mini",          "hosted"),
    "deepseek-flash":    ("deepseek-v4-flash",     "hosted"),
    "mistral-small":     ("mistral-small-latest",  "hosted"),
    # self-host / buy compute (amortized capex). granite also = watsonx quality proxy.
    "granite-8b":        ("granite4.1:8b",         "selfhost"),
    "granite-30b":       ("granite4.1:30b",        "selfhost"),
    "qwen-4b":           ("qwen3.5:4b",            "selfhost"),
    "qwen-27b":          ("qwen3.6:27b",           "selfhost"),
    "gemma-e4b":         ("gemma4:e4b",            "selfhost"),
    "gpt-oss-120b":      ("gpt-oss:120b",          "selfhost"),
    # Claude anchor (reuses claude_pool_cache where pids align)
    "haiku":             ("claude-haiku-4.5",      "anchor"),
    "sonnet":            ("claude-sonnet-4.6",     "anchor"),
    # Big OSS via Ollama Cloud (off the contended local GB10): quality reference for the
    # "buy compute" path that the local GB10 was too saturated/slow to measure.
    "gpt-oss-120b-cloud": ("gpt-oss:120b-cloud",  "cloud"),
    "gpt-oss-20b-cloud":  ("gpt-oss:20b-cloud",   "cloud"),
    "qwen-cloud":         ("qwen3.5:cloud",        "cloud"),
    "gemma-31b-cloud":    ("gemma4:31b-cloud",     "cloud"),
    # Frontier OSS via Ollama Cloud: how good can OSS get (can any beat Sonnet)?
    "deepseek-pro-cloud": ("deepseek-v4-pro:cloud", "cloud"),
    "qwen-397b-cloud":    ("qwen3.5:397b-cloud",   "cloud"),
    "glm-cloud":          ("glm-5.2:cloud",        "cloud"),
    "nemotron-ultra-cloud": ("nemotron-3-ultra:cloud", "cloud"),
    "minimax-cloud":      ("minimax-m3:cloud",     "cloud"),
    "kimi-cloud":         ("kimi-k2.7-code:cloud", "cloud"),
    "qwen-coder-480b-cloud": ("qwen3-coder:480b-cloud", "cloud"),
    # Mid/small OSS via Ollama Cloud (fill the frontier between Granite-8B and the giants).
    "ministral-3b-cloud":  ("ministral-3:3b-cloud",   "cloud"),
    "ministral-8b-cloud":  ("ministral-3:8b-cloud",   "cloud"),
    "ministral-14b-cloud": ("ministral-3:14b-cloud",  "cloud"),
    "devstral-24b-cloud":  ("devstral-small-2:24b-cloud", "cloud"),
    "nemotron-nano-30b-cloud": ("nemotron-3-nano:30b-cloud", "cloud"),
    "gemma3-4b-cloud":     ("gemma3:4b-cloud",        "cloud"),
    "gemma3-12b-cloud":    ("gemma3:12b-cloud",       "cloud"),
    "gemma3-27b-cloud":    ("gemma3:27b-cloud",       "cloud"),
}

# answer-token budgets per slice. Reasoning models (deepseek, qwen, gpt-oss) spend most
# of the budget on hidden thinking before the answer, so the hard slices need generous
# headroom or the answer is truncated (an artifact, not low quality).
ANSWER_TOKENS = {"gsm8k": 4096, "mmlu": 4096, "arc": 2048, "mbpp": 4096,
                 "gpqa": 8192, "mmlupro": 8192, "json": 768}
# Non-reasoning models (granite, gemma) answer in a few hundred tokens and have no hidden
# thinking, so the big reasoning headroom only invites runaway generation (granite4.1:30b
# looped toward 8192 tokens on hard MMLU-Pro items and wedged the GB10). Cap them.
REASONING = {"deepseek-flash", "qwen-4b", "qwen-27b", "gpt-oss-120b",
             "gpt-oss-120b-cloud", "gpt-oss-20b-cloud", "qwen-cloud", "deepseek-pro-cloud",
             "qwen-397b-cloud", "glm-cloud", "nemotron-ultra-cloud", "minimax-cloud",
             "nemotron-nano-30b-cloud"}
NONREASONING_CAP = 2048


def budget_for(alias, eval_name):
    b = ANSWER_TOKENS.get(eval_name, 1024)
    if alias not in REASONING and MODELS[alias][1] in ("selfhost", "cloud"):
        return min(b, NONREASONING_CAP)
    return b


def _key(alias, pid):
    return hashlib.sha1(f"{alias}|{pid}".encode()).hexdigest()


def call_model(model, messages, max_tokens, cache_id):
    """Like bcp.call_model but also records wall latency and tokens/sec (for throughput)."""
    os.makedirs(CACHE, exist_ok=True)
    cf = os.path.join(CACHE, cache_id + ".json")
    if os.path.exists(cf):
        return json.load(open(cf))
    body = json.dumps({"model": model, "messages": messages, "max_tokens": max_tokens,
                       "temperature": 0.0}).encode()
    req = urllib.request.Request(BASE.rstrip("/") + "/chat/completions", data=body,
                                 headers={"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"})
    last = None
    for attempt in range(7):
        try:
            t0 = time.time()
            with urllib.request.urlopen(req, timeout=600) as r:
                cost_hdr = r.headers.get("x-litellm-response-cost")
                d = json.load(r)
            latency = time.time() - t0
            break
        except Exception as e:
            last = e
            if attempt == 6:
                raise
            time.sleep(min(2 ** attempt, 20))
    text = d["choices"][0]["message"]["content"] or ""
    usage = d.get("usage", {})
    pt, ct = usage.get("prompt_tokens", 0), usage.get("completion_tokens", 0)
    cost = float(cost_hdr) if cost_hdr else 0.0  # measured hosted cost; 0 for self-host
    out = {"text": text, "prompt_tokens": pt, "completion_tokens": ct, "cost": cost,
           "latency": round(latency, 3), "tps": round(ct / latency, 2) if latency > 0 else 0.0}
    json.dump(out, open(cf, "w"))
    return out


# ------------------------------------------------------------------ extra loaders
def load_gpqa(n):
    from datasets import load_dataset
    ds = load_dataset("hendrydong/gpqa_diamond_mc", split="test").shuffle(seed=13).select(range(min(n, 198)))
    rows = []
    for i, r in enumerate(ds):
        gold = re.search(r"\\boxed\{([A-D])\}", r["solution"])
        if not gold:
            continue
        rows.append({"id": f"gpqa-{i}", "eval_name": "gpqa", "grade": "mc", "gold": gold.group(1),
                     "prompt": r["problem"] + "\n\nOn the final line write exactly 'Answer: <letter>'."})
    return rows


def load_mmlupro(n):
    from datasets import load_dataset
    ds = load_dataset("TIGER-Lab/MMLU-Pro", split="test").shuffle(seed=13).select(range(n))
    letters = "ABCDEFGHIJ"
    rows = []
    for i, r in enumerate(ds):
        ch = "\n".join(f"{letters[j]}. {c}" for j, c in enumerate(r["options"]))
        gold = r["answer"] if r["answer"] in letters else letters[int(r["answer_index"])]
        rows.append({"id": f"mmlupro-{i}", "eval_name": "mmlupro", "grade": "mc", "gold": gold,
                     "prompt": f"{r['question']}\n{ch}\n\nOn the final line write exactly 'Answer: <letter>'."})
    return rows


# Synthetic strict-JSON-to-schema probe: the structured-output reliability gate. Each item
# gives a short record in prose and asks for a flat JSON object with typed keys. Graded by
# json.loads (after fence-stripping) + required keys present with the right Python types.
_JSON_FIRST = ["Ada", "Liang", "Priya", "Mateo", "Yuki", "Omar", "Nina", "Tomas", "Fatima", "Wei"]
_JSON_LAST = ["Reyes", "Okafor", "Sato", "Nguyen", "Haddad", "Kowalski", "Mbeki", "Larsson", "Devi", "Costa"]
_JSON_CITY = ["Austin", "Lisbon", "Nairobi", "Osaka", "Bogota", "Tallinn", "Amman", "Perth", "Accra", "Quito"]


def load_json(n):
    import random
    rng = random.Random(13)
    rows = []
    for i in range(n):
        fn, ln = rng.choice(_JSON_FIRST), rng.choice(_JSON_LAST)
        age = rng.randint(19, 71)
        city = rng.choice(_JSON_CITY)
        active = rng.choice([True, False])
        score = round(rng.uniform(0, 100), 1)
        text = (f"{fn} {ln} is {age} years old and lives in {city}. "
                f"Their account is currently {'active' if active else 'inactive'} "
                f"and their loyalty score is {score}.")
        schema = {"first_name": str, "last_name": str, "age": int, "city": str,
                  "active": bool, "loyalty_score": (int, float)}
        prompt = (f"{text}\n\nExtract this into a single JSON object with exactly these keys: "
                  "first_name (string), last_name (string), age (integer), city (string), "
                  "active (boolean), loyalty_score (number). "
                  "Output ONLY the JSON object, no markdown fences, no commentary.")
        rows.append({"id": f"json-{i}", "eval_name": "json", "grade": "json",
                     "schema_keys": list(schema.keys()), "prompt": prompt,
                     "expected": {"first_name": fn, "last_name": ln, "age": age,
                                  "city": city, "active": active}})
    return rows


def grade_mc_j(text, gold):
    """Multiple-choice grader extended to A-J (for MMLU-Pro)."""
    m = re.search(r"Answer:\s*[\(<\[]?\s*([A-J])\b", text, re.I)
    if m:
        return int(m.group(1).upper() == gold.upper())
    cands = re.findall(r"\b([A-J])\b", text.strip().upper())
    return int(bool(cands) and cands[-1] == gold.upper())


def grade_json(text, schema_keys):
    """Strict-JSON gate: parse + all required keys present with correct types."""
    s = re.sub(r"^```(?:json)?|```$", "", text.strip(), flags=re.M).strip()
    m = re.search(r"\{.*\}", s, re.DOTALL)
    if not m:
        return 0
    try:
        obj = json.loads(m.group(0))
    except Exception:
        return 0
    if not isinstance(obj, dict):
        return 0
    types = {"first_name": str, "last_name": str, "age": int, "city": str,
             "active": bool, "loyalty_score": (int, float)}
    for k in schema_keys:
        if k not in obj:
            return 0
        exp = types.get(k, object)
        # bool is a subclass of int; guard so age=True does not pass.
        if exp is int and isinstance(obj[k], bool):
            return 0
        if not isinstance(obj[k], exp):
            return 0
    return 1


def grade(row, text):
    g = row["grade"]
    if g == "numeric":
        return bcp.grade_numeric(text, row["gold"])
    if g == "mc":
        return grade_mc_j(text, row["gold"])
    if g == "code":
        return bcp.grade_code(text, row["tests"], row.get("setup", ""))
    if g == "json":
        return grade_json(text, row["schema_keys"])
    return 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n-gsm8k", type=int, default=120)
    ap.add_argument("--n-mmlu", type=int, default=150)
    ap.add_argument("--n-arc", type=int, default=100)
    ap.add_argument("--n-mbpp", type=int, default=80)
    ap.add_argument("--n-gpqa", type=int, default=150)
    ap.add_argument("--n-mmlupro", type=int, default=150)
    ap.add_argument("--n-json", type=int, default=50)
    ap.add_argument("--pilot", action="store_true", help="small counts for a fast first pass")
    ap.add_argument("--models", default="", help="comma-separated alias subset (default all)")
    ap.add_argument("--out", default=os.path.join(DATA, "provider_pool.parquet"))
    args = ap.parse_args()
    if not KEY:
        sys.exit("set LITEFORGE_API_KEY")
    bcp.KEY = KEY  # graders' judge calls (unused here) and any bcp calls share the key

    if args.pilot:
        args.n_gsm8k, args.n_mmlu, args.n_arc, args.n_mbpp = 60, 60, 60, 60
        args.n_gpqa, args.n_mmlupro, args.n_json = 80, 80, 40

    aliases = [a.strip() for a in args.models.split(",") if a.strip()] or list(MODELS)
    for a in aliases:
        if a not in MODELS:
            sys.exit(f"unknown model alias {a}; known: {list(MODELS)}")

    prompts = []
    if args.n_gsm8k: prompts += bcp.load_gsm8k(args.n_gsm8k)
    if args.n_mmlu: prompts += bcp.load_mmlu(args.n_mmlu)
    if args.n_arc: prompts += bcp.load_arc(args.n_arc)
    if args.n_mbpp: prompts += bcp.load_mbpp(args.n_mbpp)
    if args.n_gpqa: prompts += load_gpqa(args.n_gpqa)
    if args.n_mmlupro: prompts += load_mmlupro(args.n_mmlupro)
    if args.n_json: prompts += load_json(args.n_json)
    print(f"loaded {len(prompts)} prompts: "
          + str(pd.Series([p['eval_name'] for p in prompts]).value_counts().to_dict()))
    print(f"models ({len(aliases)}): {aliases}")

    def fetch_one(alias, row):
        model, _ = MODELS[alias]
        try:
            return call_model(model, [{"role": "user", "content": row["prompt"]}],
                              budget_for(alias, row["eval_name"]), _key(alias, row["id"]))
        except Exception as e:
            print(f"  ! {alias} {row['id']} failed: {str(e)[:60]}")
            return None

    # Fetch phase. Hosted/anchor models: high concurrency across all (model, prompt) pairs.
    # Self-host models: one model at a time (stays warm on the single GB10), modest
    # in-model concurrency so the GPU pipelines without thrashing model reloads.
    # cloud models are remote and handle concurrency, so fetch them with the hosted group.
    hosted = [a for a in aliases if MODELS[a][1] in ("hosted", "anchor", "cloud")]
    selfhost = [a for a in aliases if MODELS[a][1] == "selfhost"]

    if hosted:
        pairs = [(a, row) for a in hosted for row in prompts]
        done = 0
        print(f"fetching {len(pairs)} hosted/anchor calls (concurrent)...")
        with ThreadPoolExecutor(max_workers=12) as ex:
            for _ in ex.map(lambda p: fetch_one(*p), pairs):
                done += 1
                if done % 200 == 0:
                    print(f"  hosted {done}/{len(pairs)}")

    for alias in selfhost:
        print(f"fetching self-host {alias} ({MODELS[alias][0]}), {len(prompts)} calls...")
        t0 = time.time()
        with ThreadPoolExecutor(max_workers=4) as ex:
            list(ex.map(lambda row: fetch_one(alias, row), prompts))
        print(f"  {alias} done in {time.time()-t0:.0f}s")

    # Grade phase (mbpp subprocesses parallelize; everything else is local + cheap).
    def grade_row(row):
        rec = {"sample_id": row["id"], "eval_name": row["eval_name"], "prompt": row["prompt"]}
        for alias in aliases:
            r = fetch_one(alias, row)  # cached
            if r is None:  # model errored / unavailable; leave its columns absent
                continue
            rec[alias] = float(grade(row, r["text"]))
            rec[f"{alias}|total_cost"] = r["cost"]
            rec[f"{alias}|pt"] = r["prompt_tokens"]
            rec[f"{alias}|ct"] = r["completion_tokens"]
            rec[f"{alias}|latency"] = r.get("latency", 0.0)
            rec[f"{alias}|tps"] = r.get("tps", 0.0)
        return rec

    with ThreadPoolExecutor(max_workers=8) as ex:
        records = list(ex.map(grade_row, prompts))

    df = pd.DataFrame(records)
    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    df.to_parquet(args.out)
    present = [a for a in aliases if a in df.columns]
    missing = [a for a in aliases if a not in df.columns]
    if missing:
        print(f"  (no data for: {missing})")
    print(f"\nwrote {args.out}  ({len(df)} rows)")
    print(f"{'model':18s} {'acc':>6s} {'hosted$/call':>13s} {'tps':>7s}")
    for a in present:
        tps = df[f"{a}|tps"][df[f"{a}|tps"] > 0]
        print(f"{a:18s} {df[a].mean():6.3f} {df[f'{a}|total_cost'].mean():13.6f} "
              f"{(tps.mean() if len(tps) else 0):7.1f}")
    print("\nper-benchmark accuracy:")
    print(df.groupby("eval_name")[present].mean().round(3).to_string())


if __name__ == "__main__":
    main()
