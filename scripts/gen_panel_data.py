#!/usr/bin/env python3
"""Generate a multi-signal routing dataset for the panel router.

Each example carries four coherent signal labels plus a synthetic context that
exercises the structured feature extractor, and a rule-derived capability group
(the fusion target):

    {
      "text": "...prompt (may include a code/diff/error/file-list context block)...",
      "task_type": "code", "difficulty": "hard", "reasoning_depth": "deep",
      "context_demand": "high",
      "feats": {"ctx_tokens":.., "n_files":.., "has_code":.., "has_diff":.., "has_error":..},
      "group": "reasoning"
    }

Experts train on text -> one signal each. The fusion mapper trains on
(signal one-hots ++ structured features) -> group. Synthetic, so in-distribution
accuracy is high; mix in real labeled traffic for production.

    python scripts/gen_panel_data.py --n 12000 --out panel.jsonl
"""
import argparse
import json
import random

TASK_TYPES = ["chitchat", "qa", "code", "debug", "refactor", "math", "reasoning", "writing", "data"]
DIFFICULTY = ["easy", "medium", "hard"]
REASONING = ["shallow", "deep"]
CONTEXT = ["low", "high"]
GROUPS = ["chat", "code", "reasoning", "long_context", "general"]

WORDS = ["river", "matrix", "kernel", "ledger", "quartz", "syntax", "vector", "harbor", "cipher", "glacier"]
LANGS = ["python", "rust", "typescript", "go", "java", "sql"]
CONCEPTS = ["OAuth2", "indexing", "caching", "sharding", "pub/sub", "memoization", "transactions",
            "load balancing", "vectorization", "garbage collection"]


def code_block(lang, lines):
    body = "\n".join(f"    line_{i} = compute({i})" for i in range(lines))
    return f"```{lang}\n{body}\n```"


def diff_block(files):
    out = []
    for f in range(files):
        out.append(f"diff --git a/src/mod{f}.{random.choice(['rs','py','ts'])} b/src/mod{f}.x\n@@ -1,4 +1,6 @@\n-old()\n+new()")
    return "\n".join(out)


def traceback_block():
    return ('Traceback (most recent call last):\n  File "app.py", line 42, in run\n'
            "    handler()\nValueError: invalid state")


def file_list(n):
    return "Files:\n" + "\n".join(f"- src/module_{i}.{random.choice(LANGS)}" for i in range(n))


# Archetype: (task, difficulty, reasoning, context, prompt templates, context-block kind)
# context-block kind in {none, code, code+err, bigcode, diff, files}
ARCHETYPES = [
    ("chitchat", "easy", "shallow", "low",
     ["hi", "hey there", "thanks!", "good morning", "how's it going?", "yo, what's up",
      "hi, working on {concept} today", "good morning! ready for {w}?", "thanks for the {concept} help",
      "hey, taking a break from {w}", "happy to be coding {concept}", "lol nice, what about {w}?",
      "appreciate it!", "great, thanks a ton for {concept}", "morning, how are you doing?",
      "cheers, that {w} tip helped", "you around?", "quick hello before {concept}"], "none"),
    ("qa", "easy", "shallow", "low",
     ["what's the capital of {w}?", "what is {n1} plus {n2}?", "define '{w}'",
      "is {n1} prime?", "spell '{w}'", "what year is it?", "what's {n1} times {n2}?",
      "synonym for '{w}'?", "what does {concept} stand for?", "round {n1}.5 please",
      "plural of '{w}'?", "is {n1} even?", "translate '{w}' to French", "abbreviation for {concept}?"], "none"),
    ("writing", "medium", "shallow", "low",
     ["write a short poem about {w}", "draft a friendly email about {concept}",
      "rewrite this sentence to be clearer", "give me three taglines for {w}",
      "write a tweet announcing {concept}", "compose a haiku about {w}",
      "draft release notes for the {concept} feature", "write a product blurb for {w}",
      "summarize {concept} in two sentences", "write a polite decline email about {w}",
      "craft a slogan for {concept}", "outline a blog post on {w}"], "none"),
    ("code", "medium", "shallow", "low",
     ["write a {lang} function to reverse a string", "implement bubble sort in {lang}",
      "add a CLI flag to this {lang} script", "write a regex for an email"], "code"),
    ("code", "medium", "deep", "high",
     ["refactor this {lang} module to remove duplication", "add a feature across these files"], "bigcode"),
    ("debug", "medium", "shallow", "high",
     ["fix this {lang} error", "why does this throw?", "debug this failing test"], "code+err"),
    ("debug", "hard", "deep", "high",
     ["diagnose this intermittent deadlock", "find the root cause of this race condition"], "code+err"),
    ("refactor", "hard", "deep", "high",
     ["refactor this {n1}-file service to remove the circular dependency",
      "split this monolith module and keep tests green"], "diff"),
    ("math", "hard", "deep", "low",
     ["prove there are infinitely many primes of the form 4k+3",
      "derive the gradient of softmax cross-entropy", "prove Dijkstra is correct",
      "show that sqrt({n1}) is irrational", "derive a closed form for the sum of the first {n1} squares",
      "prove the {w} inequality by induction", "prove that {concept} converges",
      "derive the eigenvalues of this {n1}x{n1} matrix", "prove this recurrence has complexity O(n log n)",
      "show the {w} series diverges", "prove by contradiction that {n1} is not a perfect square",
      "derive Bayes' rule from the definition of conditional probability",
      "prove the triangle inequality for the {w} norm", "establish a tight bound on this integral"], "none"),
    ("reasoning", "hard", "deep", "low",
     ["design a Byzantine fault tolerant consensus protocol and prove safety",
      "architect a multi-region rate limiter and defend the tradeoffs"], "none"),
    ("reasoning", "hard", "deep", "high",
     ["given these {n1} files, design a zero-downtime migration with rollback",
      "review this large change for correctness and propose a refactor"], "files"),
    ("data", "medium", "shallow", "low",
     ["write a SQL query for the top 5 {w}s", "turn this list into CSV", "parse this JSON for {w}",
      "group these {w} rows by month and sum the totals", "write a pandas snippet to dedupe {w}s",
      "join the {w} and {concept} tables on id", "pivot this table by {w}",
      "extract the {w} field from each record", "write a SQL query counting {w}s per day",
      "normalize these {w} values to 0..1", "filter rows where {w} > {n1}",
      "aggregate {w}s into a histogram with {n2} buckets"], "none"),
    ("qa", "medium", "deep", "low",
     ["explain how {concept} works and when to use it", "compare {concept} vs {concept2}"], "none"),
]

PREFIX = ["", "", "please ", "hey, ", "can you ", "I need help: ", "quick: "]


def fill(t):
    return t.format(n1=random.randint(2, 900), n2=random.randint(2, 99), w=random.choice(WORDS),
                    lang=random.choice(LANGS), concept=random.choice(CONCEPTS),
                    concept2=random.choice(CONCEPTS))


def build_context(kind):
    """Return (context_text, n_files, has_code, has_diff, has_error)."""
    if kind == "none":
        return "", 0, 0, 0, 0
    if kind == "code":
        return "\n\n" + code_block(random.choice(LANGS), random.randint(3, 8)), 1, 1, 0, 0
    if kind == "bigcode":
        nf = random.randint(4, 9)
        blocks = "\n\n".join(code_block(random.choice(LANGS), random.randint(15, 40)) for _ in range(2))
        return "\n\n" + file_list(nf) + "\n\n" + blocks, nf, 1, 0, 0
    if kind == "code+err":
        return "\n\n" + code_block(random.choice(LANGS), random.randint(4, 10)) + "\n\n" + traceback_block(), 1, 1, 0, 1
    if kind == "diff":
        nf = random.randint(3, 8)
        return "\n\n" + file_list(nf) + "\n\n" + diff_block(nf), nf, 1, 1, 0
    if kind == "files":
        nf = random.randint(4, 10)
        return "\n\n" + file_list(nf), nf, 0, 0, 0
    return "", 0, 0, 0, 0


def approx_tokens(s):
    return max(1, len(s) // 4)


def fusion_group(task, diff, reason, ctx, feats):
    """Transparent rule the fusion mapper learns (signals+features -> group)."""
    big_ctx = ctx == "high" or feats["n_files"] >= 4 or feats["ctx_tokens"] > 1500
    if reason == "deep" and diff == "hard" and not big_ctx:
        return "reasoning"
    if reason == "deep" and diff == "hard" and big_ctx:
        # hard + deep but lots of context: still reasoning-grade
        return "reasoning"
    if big_ctx:
        return "long_context"
    if task in ("code", "debug", "refactor", "data"):
        return "code"
    if task in ("chitchat", "qa") and diff == "easy":
        return "chat"
    return "general"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=12000)
    ap.add_argument("--seed", type=int, default=21)
    ap.add_argument("--out", default="-")
    args = ap.parse_args()
    random.seed(args.seed)

    rows, seen = [], set()
    tries = 0
    while len(rows) < args.n and tries < args.n * 60:
        tries += 1
        task, diff, reason, ctx, templates, kind = random.choice(ARCHETYPES)
        prompt = (random.choice(PREFIX) + fill(random.choice(templates))).strip()
        cblock, n_files, has_code, has_diff, has_error = build_context(kind)
        text = prompt + cblock
        if text in seen:
            continue
        seen.add(text)
        feats = {
            "ctx_tokens": approx_tokens(text),
            "n_files": n_files,
            "has_code": has_code,
            "has_diff": has_diff,
            "has_error": has_error,
        }
        group = fusion_group(task, diff, reason, ctx, feats)
        rows.append({
            "text": text, "task_type": task, "difficulty": diff,
            "reasoning_depth": reason, "context_demand": ctx, "feats": feats, "group": group,
        })
    random.shuffle(rows)

    import sys
    f = open(args.out, "w") if args.out != "-" else sys.stdout
    for r in rows:
        f.write(json.dumps(r) + "\n")
    if args.out != "-":
        f.close()
        from collections import Counter
        for field in ("task_type", "difficulty", "reasoning_depth", "context_demand", "group"):
            c = Counter(r[field] for r in rows)
            print(f"{field:16s} {dict(c)}")
        print(f"wrote {len(rows)} rows to {args.out}")


if __name__ == "__main__":
    main()
