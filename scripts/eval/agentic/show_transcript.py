#!/usr/bin/env python3
"""Pretty-print a cached agentic episode transcript so we can SEE why a model passed/failed.

    python scripts/eval/agentic/show_transcript.py --env airline --task 2 --model claude-opus-4.7 --trial 0
    python scripts/eval/agentic/show_transcript.py --env airline --task 2 --pool gemini --tier strong --trial 0
"""
import argparse
import hashlib
import json
import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, _HERE)
from run_agent import POOLS, CACHE, PROTO  # noqa: E402


def _key(*parts):
    return hashlib.sha1("|".join(map(str, parts)).encode()).hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bench", default="tau")
    ap.add_argument("--env", default="airline")
    ap.add_argument("--task", type=int, required=True)
    ap.add_argument("--model", default=None)
    ap.add_argument("--pool", default=None, choices=list(POOLS))
    ap.add_argument("--tier", default=None, choices=["weak", "medium", "strong"])
    ap.add_argument("--trial", type=int, default=0)
    ap.add_argument("--proto", default=PROTO, help=f"protocol generation (default {PROTO}; use v2 for old diagnostic data)")
    ap.add_argument("--full", action="store_true", help="print full content, not snippets")
    a = ap.parse_args()
    model = a.model or POOLS[a.pool][a.tier]
    cf = os.path.join(CACHE, _key(a.bench, a.proto, a.env, a.task, model, a.trial) + ".json")
    if not os.path.exists(cf):
        sys.exit(f"no cached episode at {cf}\n  (model={model} task={a.task} trial={a.trial})")
    d = json.load(open(cf))
    n = 99999 if a.full else 600
    print(f"=== {a.bench}/{a.env} task {a.task} | {model} | trial {a.trial} ===")
    print(f"success={d.get('success')} reward={d.get('reward')} finish={d.get('finish','?')} "
          f"steps={d.get('steps')} tool_calls={d.get('tool_calls')} tool_errors={d.get('tool_errors')} "
          f"cost=${d.get('cost',0):.4f} out_tokens={d.get('completion_tokens')}")
    tr = d.get("transcript")
    if not tr:
        print("\n(no transcript stored — this episode predates transcript capture; re-run it)")
        return
    print(f"--- {len(tr)} turns ---")
    for m in tr:
        role = m.get("role")
        if role == "assistant":
            tcs = m.get("tool_calls") or []
            if tcs:
                fn = tcs[0].get("function", {})
                print(f"\n[ASSISTANT -> tool] {fn.get('name')}({fn.get('arguments','')[:n]})")
                if m.get("content"):
                    print(f"   (thought: {str(m['content'])[:n]})")
            else:
                print(f"\n[ASSISTANT -> user] {str(m.get('content'))[:n]}")
        elif role == "tool":
            print(f"[tool:{m.get('name')}] {str(m.get('content'))[:n]}")
        elif role == "user":
            print(f"\n[USER] {str(m.get('content'))[:n]}")
        elif role == "system":
            print(f"[system] ({len(str(m.get('content')))} chars policy/wiki)")


if __name__ == "__main__":
    main()
