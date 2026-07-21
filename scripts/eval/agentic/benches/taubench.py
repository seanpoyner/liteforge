#!/usr/bin/env python3
"""tau-bench adapter: drive a routing-tier model through the tau-bench retail/airline env
(real Python tools + LLM-simulated user) and grade by the env's DB-state reward.

The tau-bench user simulator runs through OUR gateway too (litellm provider=openai pointed at
the LiteLLM proxy), so no external API is used. Run directly for a small 3-tier validation:

    python scripts/eval/agentic/benches/taubench.py --env retail --n 3
"""
import argparse
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from run_agent import call_chat, run_cached, TIERS, CACHE, _key, PROTO  # noqa: E402

# User-simulator model, via the gateway. MUST be a strong instruction-follower: it runs every
# turn and grades the rollout by staying in character + enforcing the task's constraints. A weak
# user-sim (haiku) broke character (hallucinated confirmations after tool errors, aborting the
# agent's recovery) and approved constraint-violating bookings -> use sonnet-class.
USER_SIM = os.environ.get("TAU_USER_MODEL", "claude-sonnet-4.6")


# Generic, model-agnostic agent scaffold prepended to the domain wiki for EVERY tier (PROTO v4).
# It encodes the operating loop only - it names NO domain-specific tool and gives NO task hints,
# so it cannot favour any one model. Purpose: stop spurious give-ups (e.g. claiming a capability
# is unavailable without checking the tool list, or transferring to a human prematurely).
TAU_SCAFFOLD = """# Agent operating instructions

You are a customer-service agent who completes the user's request end to end by calling the available tools.

- Take exactly ONE tool action per step, then wait for its result before deciding the next action.
- Before telling the user that something cannot be done, review the FULL list of tools available to you. Do not assume a capability is missing - check your tools first.
- Treat transferring to a human as a LAST RESORT: only do so after you have exhausted the available tools and confirmed that none of them can address the request.
- Keep working until the task is genuinely complete or genuinely impossible with the given tools.

Follow the domain policy below.

---

"""


def _setup_gateway_env():
    key = os.environ.get("LITEFORGE_API_KEY", "")
    os.environ["OPENAI_API_KEY"] = key
    os.environ["OPENAI_API_BASE"] = os.environ.get("ROUTER_EVAL_BASE_URL", "http://10.8.0.6:4000/v1")
    os.environ["OPENAI_BASE_URL"] = os.environ["OPENAI_API_BASE"]


def run_tau_task(tier, env_name, task_index, max_steps=30, max_tokens=16384, temperature=0.0):
    # max_tokens=16384: thinking/verbose models (opus) need headroom or their reasoning eats the
    # 4096 budget and the response is truncated (finish_reason=length) before the tool_call is
    # emitted -> empty turns -> loops to max_steps. Probe: opus task3 truncated_calls 5 -> 0 at 16384.
    """Run one tau-bench task with the given tier; return success/reward/steps + full transcript.
    Matches the official tau agent: ONE action (first tool_call) per turn, max_num_steps=30.
    temperature>0 + multiple trials gives a per-task success-rate estimate (cuts noise)."""
    from tau_bench.envs import get_env
    from tau_bench.types import Action
    env = get_env(env_name, user_strategy="llm", user_model=USER_SIM, user_provider="openai",
                  task_split="test", task_index=task_index)
    reset = env.reset(task_index=task_index)
    # PROTO v4: prepend the shared model-agnostic scaffold to the domain wiki, applied to EVERY tier.
    messages = [{"role": "system", "content": TAU_SCAFFOLD + env.wiki},
                {"role": "user", "content": str(reset.observation)}]
    tools = env.tools_info
    cost = 0.0; pt = 0; ct = 0; steps = 0; n_calls = 0; n_err = 0; reward = 0.0
    finish = "max_steps"; n_truncated = 0
    for _ in range(max_steps):
        steps += 1
        msg, c, usage = call_chat(TIERS[tier], messages, tools=tools, max_tokens=max_tokens,
                                  temperature=temperature)
        cost += c; pt += usage.get("prompt_tokens", 0); ct += usage.get("completion_tokens", 0)
        if usage.get("finish_reason") == "length":
            n_truncated += 1  # response cut off before the model finished (thinking ate the budget)
        tcs = msg.get("tool_calls") or []
        if tcs:
            # Official tau protocol: exactly ONE action (the first tool_call) per turn.
            tc = tcs[0]; n_calls += 1
            messages.append({"role": "assistant", "content": msg.get("content"), "tool_calls": [tc]})
            fn = tc.get("function", {}); name = fn.get("name", "")
            try:
                kwargs = json.loads(fn.get("arguments") or "{}")
            except Exception:
                kwargs = {}; n_err += 1
            try:
                resp = env.step(Action(name=name, kwargs=kwargs))
                obs, reward, done = resp.observation, resp.reward, resp.done
            except Exception as e:
                obs = f"ERROR: {e}"; n_err += 1; done = False
            messages.append({"role": "tool", "tool_call_id": tc.get("id", ""), "name": name,
                             "content": str(obs)[:6000]})
            if done:
                finish = "tool_done"; break
        else:
            # no tool call -> respond to the user, advancing the user simulator
            messages.append({"role": "assistant", "content": msg.get("content")})
            resp = env.step(Action(name="respond", kwargs={"content": msg.get("content") or ""}))
            reward = resp.reward
            messages.append({"role": "user", "content": str(resp.observation)})
            if resp.done:
                finish = "responded"; break
    return {"success": float(reward >= 0.999), "reward": float(reward), "cost": cost,
            "prompt_tokens": pt, "completion_tokens": ct, "steps": steps,
            "tool_calls": n_calls, "tool_errors": n_err, "finish": finish,
            "truncated_calls": n_truncated, "transcript": messages}


def run_cached_tau(tier, env_name, task_index, trial=0, **kw):
    os.makedirs(CACHE, exist_ok=True)
    cf = os.path.join(CACHE, _key("tau", PROTO, env_name, task_index, TIERS[tier], trial) + ".json")
    if os.path.exists(cf):
        return json.load(open(cf))
    out = run_tau_task(tier, env_name, task_index, **kw)
    json.dump(out, open(cf, "w"))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--env", default="retail", choices=["retail", "airline"])
    ap.add_argument("--n", type=int, default=3)
    ap.add_argument("--start", type=int, default=0, help="first task index (for sharding disjoint ranges)")
    ap.add_argument("--trials", type=int, default=1)
    ap.add_argument("--temp", type=float, default=None, help="agent temperature; defaults 0.0 single-trial, 0.7 multi-trial")
    ap.add_argument("--tiers", default="weak,medium,strong")
    args = ap.parse_args()
    if not os.environ.get("LITEFORGE_API_KEY"):
        sys.exit("set LITEFORGE_API_KEY")
    _setup_gateway_env()
    tiers = args.tiers.split(",")
    temp = args.temp if args.temp is not None else (0.0 if args.trials == 1 else 0.7)
    print(f"tau-bench[{args.env}] n={args.n} trials={args.trials} temp={temp} tiers={tiers} (user-sim={USER_SIM})")
    # per (task,tier) success rate over trials
    rate = {t: [] for t in tiers}; cost = {t: [] for t in tiers}
    for i in range(args.start, args.start + args.n):
        line = f"task {i:3d}: "
        for t in tiers:
            t0 = time.time()
            try:
                trs = [run_cached_tau(t, args.env, i, trial=k, temperature=temp) for k in range(args.trials)]
                sr = sum(x["success"] for x in trs) / len(trs)
                ac = sum(x["cost"] for x in trs) / len(trs)
                rate[t].append(sr); cost[t].append(ac)
                line += f"{t}={sr:.2f}(${ac:.3f},{time.time()-t0:.0f}s) "
            except Exception as e:
                line += f"{t}=ERR({str(e)[:34]}) "
        print(line, flush=True)
    print("\n=== tier summary (mean per-task success rate over trials) ===")
    for t in tiers:
        if rate[t]:
            print(f"  {t:6s} success {sum(rate[t])/len(rate[t]):.3f}  avg_cost ${sum(cost[t])/len(cost[t]):.4f}  "
                  f"tasks={len(rate[t])} trials={args.trials}")


if __name__ == "__main__":
    main()
