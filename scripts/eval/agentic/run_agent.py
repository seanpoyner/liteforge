#!/usr/bin/env python3
"""Agentic tool-calling loop against the OpenAI-compatible LiteLLM gateway.

The core of the hard agentic routing benchmark: drive a model through a multi-step
tool-calling episode with REAL tool execution, and record success/cost/tokens/steps.
Benchmark adapters (tau-bench, BFCL, SWE-bench, GAIA) provide the tools + dispatch +
grader; this module is bench-agnostic.

Run directly for a self-contained smoke test that proves native tool-calling works
end-to-end through the gateway for the routing tiers:

    python scripts/eval/agentic/run_agent.py            # smoke test (cheap + strong tier)
"""
import hashlib
import json
import os
import sys
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "data")
CACHE = os.path.join(DATA, "agentic_cache")
BASE = os.environ.get("ROUTER_EVAL_BASE_URL", "http://10.8.0.6:4000/v1")
KEY = os.environ.get("LITEFORGE_API_KEY", "")
# Episode protocol version - bump when semantics change so old caches aren't reused.
# Encodes the WHOLE protocol generation (parity + agent temp + user-sim), so cached
# episodes from a different generation are never silently mixed in.
#   v2 = parity (first tool_call/turn, max_steps=30, transcripts) BUT agent temp 0.7 + haiku
#        user-sim. Diagnostic-only: temp-0.7 slips + a too-weak user-sim drowned the signal.
#   v3 = corrected: agent temp 0 (official greedy) + claude-sonnet-4.6 user-sim (stays in
#        character, enforces task constraints).
#   v4 = v3 + a generic, model-agnostic agent scaffold prepended to the wiki for EVERY tier
#        (one action/step; review the full tool list before claiming a capability is missing;
#        transfer-to-human is a last resort). Diverges from official tau (so numbers don't match
#        the tau leaderboard) but is deployment-realistic and applied equally, so the routing
#        comparison stays fair.
#   v5 = v4 + per-call max_tokens raised 4096 -> 16384. At 4096 a thinking/verbose model (opus)
#        had its reasoning truncated (finish_reason=length) before emitting the tool_call, causing
#        empty turns + max_steps loops; this corrupted the gemini pool. v5 is the routing-eval
#        protocol. (Residual: opus still over-explores the hardest multi-step tasks - real behavior.)
PROTO = "v5"

# Routing pools (gateway aliases), weak/medium/strong. Select via ROUTING_POOL env var.
#  gemini : managed Google stack - gemma4:31b (weak) / Flash-Lite (medium) / Pro (strong)
#  ollama : self-hostable OSS stack on ollama (cloud) - 3 similar tiers
POOLS = {
    "gemini": {"weak": "gemma4:31b-cloud", "medium": "gemini-3.1-flash-lite", "strong": "claude-sonnet-4.6"},
    # heavyweight self-hostable OSS ladder (latest): ~30B / Qwen 100-400B / 400B+
    "ollama": {"weak": "nemotron-3-nano:30b-cloud", "medium": "qwen3.5:397b-cloud", "strong": "glm-5.2:cloud"},
    # gemma-weak variant: same medium/strong (cached episodes reused), stronger ~31B weak tier
    "ollama_gemma": {"weak": "gemma4:31b-cloud", "medium": "qwen3.5:397b-cloud", "strong": "glm-5.2:cloud"},
}
ACTIVE_POOL = os.environ.get("ROUTING_POOL", "gemini")
TIERS = POOLS[ACTIVE_POOL]


def _key(*parts):
    return hashlib.sha1("|".join(map(str, parts)).encode()).hexdigest()


# Per-1M (in,out) USD price fallback when the gateway omits x-litellm-response-cost
# (observed missing for claude-opus and gemini-3.1-pro). ollama-served models ~0 (self-host).
PRICES = {
    "claude-opus-4.7": (15.0, 75.0), "claude-sonnet-4.6": (3.0, 15.0),
    "gemini-3.1-pro": (2.0, 12.0), "gemini-3.1-flash-lite": (0.25, 1.50),
}

# Compute-cost PROXY for the ollama tiers ($/1M in,out tokens), since they are served free via
# the gateway ($0) and the routing eval needs a non-degenerate cost axis. Values are scaled by
# active parameter count (nemotron-30b < qwen-397b < glm-5.2), output ~4x input - the standard
# self-host shape. The absolute constants are a documented modeling assumption; only the
# monotonic ordering by tier drives the cost-quality / APGR result. Edit here to re-price.
OLLAMA_PRICES = {
    "nemotron-3-nano:30b-cloud": (0.05, 0.20),   # ~30B
    "gemma4:31b-cloud": (0.05, 0.20),            # ~31B (gemma-weak variant)
    "qwen3.5:397b-cloud": (0.50, 2.00),          # ~397B
    "glm-5.2:cloud": (0.90, 3.60),               # ~700B+ (largest tier)
}


def call_chat(model, messages, tools=None, tool_choice="auto", max_tokens=4096, temperature=0.0):
    """One gateway chat call. Returns (assistant_message_dict, cost, usage). Retries on transient."""
    body = {"model": model, "messages": messages, "max_tokens": max_tokens, "temperature": temperature}
    if tools:
        body["tools"] = tools
        body["tool_choice"] = tool_choice
    data = json.dumps(body).encode()
    req = urllib.request.Request(BASE.rstrip("/") + "/chat/completions", data=data,
                                 headers={"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"})
    for attempt in range(6):
        try:
            with urllib.request.urlopen(req, timeout=600) as r:
                cost = r.headers.get("x-litellm-response-cost")
                d = json.load(r)
            msg = d["choices"][0]["message"]
            usage = d.get("usage", {})
            usage["finish_reason"] = d["choices"][0].get("finish_reason")  # 'length' => truncated
            if cost:
                c = float(cost)
            else:  # gateway omitted cost; fall back to a known price table
                pin, pout = PRICES.get(model, (0.0, 0.0))
                c = usage.get("prompt_tokens", 0) / 1e6 * pin + usage.get("completion_tokens", 0) / 1e6 * pout
            return msg, c, usage
        except Exception:
            if attempt == 5:
                raise
            time.sleep(min(2 ** attempt, 20))


def run_episode(model, system, user, tools, dispatch, max_steps=12, max_tokens=4096):
    """Drive one agentic episode with real tool execution.

    tools: list of OpenAI tool schemas. dispatch(name, args_dict) -> str (REAL execution).
    Returns dict: final_text, messages (full transcript), steps, cost, in/out tokens,
    tool_calls (count), tool_errors (bad name / unparseable args / dispatch raised).
    """
    messages = []
    if system:
        messages.append({"role": "system", "content": system})
    messages.append({"role": "user", "content": user})
    cost = 0.0; pt = 0; ct = 0; steps = 0; n_calls = 0; n_errors = 0
    final = ""
    for _ in range(max_steps):
        steps += 1
        msg, c, usage = call_chat(model, messages, tools=tools, max_tokens=max_tokens)
        cost += c; pt += usage.get("prompt_tokens", 0); ct += usage.get("completion_tokens", 0)
        tcs = msg.get("tool_calls") or []
        # append the assistant turn verbatim (content may be null when only tool_calls)
        messages.append({"role": "assistant", "content": msg.get("content"),
                         **({"tool_calls": tcs} if tcs else {})})
        if not tcs:
            final = msg.get("content") or ""
            break
        for tc in tcs:
            n_calls += 1
            fn = tc.get("function", {})
            name = fn.get("name", "")
            try:
                args = json.loads(fn.get("arguments") or "{}")
            except Exception:
                args = {}; n_errors += 1
            try:
                result = dispatch(name, args)
            except Exception as e:
                result = f"ERROR: {e}"; n_errors += 1
            messages.append({"role": "tool", "tool_call_id": tc.get("id", ""),
                             "name": name, "content": str(result)[:6000]})
    return {"final_text": final, "messages": messages, "steps": steps, "cost": cost,
            "prompt_tokens": pt, "completion_tokens": ct, "tool_calls": n_calls,
            "tool_errors": n_errors}


def run_cached(bench, task_id, tier, system, user, tools, dispatch, **kw):
    os.makedirs(CACHE, exist_ok=True)
    cf = os.path.join(CACHE, _key(bench, task_id, TIERS[tier]) + ".json")  # key by model id
    if os.path.exists(cf):
        return json.load(open(cf))
    out = run_episode(TIERS[tier], system, user, tools, dispatch, **kw)
    json.dump(out, open(cf, "w"))
    return out


# --------------------------------------------------------------------- smoke test
def _smoke():
    if not KEY:
        sys.exit("set LITEFORGE_API_KEY")
    # a tiny REAL tool env requiring two tool calls + arithmetic the model must combine
    temps = {"paris": 12, "tokyo": 18, "denver": 7}

    def dispatch(name, args):
        if name == "get_temp":
            return json.dumps({"city": args.get("city"), "temp_c": temps.get(str(args.get("city", "")).lower(), "unknown")})
        if name == "multiply":
            return json.dumps({"product": float(args.get("a", 0)) * float(args.get("b", 0))})
        return f"ERROR: unknown tool {name}"

    tools = [
        {"type": "function", "function": {"name": "get_temp", "description": "Current temperature (C) for a city.",
            "parameters": {"type": "object", "properties": {"city": {"type": "string"}}, "required": ["city"]}}},
        {"type": "function", "function": {"name": "multiply", "description": "Multiply two numbers.",
            "parameters": {"type": "object", "properties": {"a": {"type": "number"}, "b": {"type": "number"}},
                           "required": ["a", "b"]}}},
    ]
    system = "You are a precise assistant. Use the tools; do not guess. End with 'Answer: <number>'."
    user = ("Compute 19 times 23, then add the current temperature in Paris (Celsius). "
            "Use the tools for both. Give the final number.")
    gold = 19 * 23 + temps["paris"]  # 449
    import re
    for tier in ("cheap", "strong"):
        t0 = time.time()
        try:
            r = run_episode(TIERS[tier], system, user, tools, dispatch, max_steps=8)
            m = re.search(r"Answer:\s*\$?(-?\d[\d,]*\.?\d*)", r["final_text"])
            ans = m.group(1).replace(",", "") if m else None
            ok = ans is not None and abs(float(ans) - gold) < 0.5
            print(f"{tier:6s} [{TIERS[tier]:22s}] {time.time()-t0:5.1f}s steps={r['steps']} "
                  f"calls={r['tool_calls']} err={r['tool_errors']} cost=${r['cost']:.5f} "
                  f"ans={ans} OK={ok}  (gold {gold})")
        except Exception as e:
            print(f"{tier:6s} [{TIERS[tier]:22s}] ERR {str(e)[:90]}")


if __name__ == "__main__":
    _smoke()
