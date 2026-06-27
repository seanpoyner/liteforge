#!/usr/bin/env python3
"""Rebalance the N=50 ollama-airline run: once the fast shards (start 20/30/40) finish, keep a
pool of helper workers draining the remaining unfinished tasks (highest index first, so they
meet the still-running slow shards 0/10 in the middle). Cache is per-(task,tier,trial) and
run_cached_tau skips existing files, so overlap is at worst one redundant episode."""
import hashlib
import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from run_agent import CACHE, POOLS  # noqa: E402

POOL = POOLS["ollama"]
N = 50
CAP = 4  # concurrent helpers (slow shards 0/10 keep running alongside)


def _key(*p):
    return hashlib.sha1("|".join(map(str, p)).encode()).hexdigest()


def done(t):
    return all(os.path.exists(os.path.join(CACHE, _key("tau", "v5", "airline", t, POOL[tier], k) + ".json"))
               for tier in POOL for k in range(3))


def fast_shards_running():
    ps = subprocess.run(["ps", "-eo", "args"], capture_output=True, text=True).stdout
    return any(f"--start {s} " in ps for s in ("20", "30", "40"))


def main():
    print("rebalance: waiting for fast shards (start 20/30/40) to finish...", flush=True)
    while fast_shards_running():
        time.sleep(30)
    print(f"fast shards done; draining remaining tasks with up to {CAP} helpers", flush=True)
    env = dict(os.environ); env["ROUTING_POOL"] = "ollama"
    running = {}  # task -> Popen
    while True:
        for t, p in list(running.items()):
            if p.poll() is not None:
                del running[t]
        todo = [t for t in range(N) if not done(t) and t not in running]
        if not todo and not running:
            break
        todo.sort(reverse=True)  # high end first; slow shards 0/10 work the low end
        while todo and len(running) < CAP:
            t = todo.pop(0)
            f = open(f"logs_v5_rebalance_t{t}.txt", "w")
            running[t] = subprocess.Popen(
                [".venv/bin/python", "benches/taubench.py", "--env", "airline", "--n", "1",
                 "--start", str(t), "--tiers", "weak,medium,strong", "--trials", "3", "--temp", "0.0"],
                stdout=f, stderr=subprocess.STDOUT, env=env)
            print(f"  helper -> task {t} ({len(running)} running)", flush=True)
        time.sleep(20)
    nd = sum(done(t) for t in range(N))
    print(f"REBALANCE COMPLETE: {nd}/{N} tasks done", flush=True)


if __name__ == "__main__":
    main()
