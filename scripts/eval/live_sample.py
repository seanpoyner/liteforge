#!/usr/bin/env python3
"""Small live end-to-end sanity check through the deployed router-panel + LiteLLM.

For a handful of prompts: ask router-panel (via LiteLLM) for capability-group
scores, map to a model, call that model live, and record the routed model + latency.
Bounded token spend. Writes scripts/eval/results/live_sample.json.
"""
import json
import os
import time
import urllib.request

BASE = os.environ.get("LITEFORGE_BASE_URL", "https://litellm.poyner.ai/v1")
KEY = os.environ["LITEFORGE_API_KEY"]
GROUP_TO_MODEL = {
    "chat": "claude-haiku-4.5", "code": "claude-sonnet-4.6",
    "reasoning": "claude-opus-4.7", "long_context": "claude-sonnet-4.6",
    "general": "claude-sonnet-4.6",
}
PROMPTS = [
    "hey, good evening!",
    "what is 12 times 8?",
    "write a python function to check if a string is a palindrome",
    "refactor this module to remove the global mutex and add tests",
    "prove that the square root of 2 is irrational",
    "design a fault-tolerant consensus protocol and argue its safety",
    "summarize the plot of a generic detective novel in two sentences",
    "debug: my recursion overflows the stack on large inputs, why?",
]


def post(path, body):
    req = urllib.request.Request(BASE.rstrip("/") + path, data=json.dumps(body).encode(),
                                 headers={"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"})
    return json.load(urllib.request.urlopen(req, timeout=120))


def main():
    rows = []
    for p in PROMPTS:
        # 1) ask the panel
        t0 = time.time()
        r = post("/chat/completions", {"model": "router-panel", "messages": [{"role": "user", "content": p}]})
        panel_ms = round((time.time() - t0) * 1000)
        content = r["choices"][0]["message"]["content"]
        parsed = json.loads(content)
        group = max(parsed["scores"], key=parsed["scores"].get)
        model = GROUP_TO_MODEL.get(group, "claude-sonnet-4.6")
        # 2) call the routed model live
        t1 = time.time()
        ans = post("/chat/completions", {"model": model,
                   "messages": [{"role": "user", "content": p}], "max_tokens": 64})
        ans_ms = round((time.time() - t1) * 1000)
        txt = ans["choices"][0]["message"]["content"]
        rows.append({"prompt": p, "group": group, "signals": parsed.get("signals"),
                     "routed_model": model, "panel_ms": panel_ms, "model_ms": ans_ms,
                     "answer_preview": txt[:90].replace("\n", " ")})
        print(f"[{group:12s} -> {model:18s}] panel {panel_ms}ms  {p[:50]}")

    json.dump(rows, open(os.path.join(os.path.dirname(__file__), "results", "live_sample.json"), "w"), indent=2)
    avg = sum(r["panel_ms"] for r in rows) / len(rows)
    print(f"\nrouted {len(rows)} prompts; avg panel latency {avg:.0f} ms; wrote results/live_sample.json")


if __name__ == "__main__":
    main()
