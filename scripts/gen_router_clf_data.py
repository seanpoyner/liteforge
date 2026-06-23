#!/usr/bin/env python3
"""Generate a varied 3-class routing dataset for the BERT-style router classifier.

Classes (difficulty -> capability tier):
    0 = easy   -> cheap
    1 = medium -> balanced
    2 = hard   -> premium

Output: JSONL {"text": "<prompt>", "label": 0|1|2}

This is synthetic, template + paraphrase generated, so a small classifier reaches
very high in-distribution accuracy. For production, mix in / replace with real
labeled traffic; held-out accuracy here measures in-distribution separability.

    python scripts/gen_router_clf_data.py --n-per-class 3000 --out data.jsonl
"""
import argparse
import json
import random

EASY = [
    "hi", "hello", "hey there", "good morning", "good evening", "thanks!",
    "thank you so much", "how are you?", "what's up?", "yo",
    "what is {n1} plus {n2}?", "what's {n1} minus {n2}?", "{n1} times {n2}?",
    "is {n1} even or odd?", "round {n1}.5 please", "what comes after {day}?",
    "capital of {country}?", "what color is the {thing}?", "spell '{w}'",
    "uppercase '{w}'", "reverse the word '{w}'", "plural of '{w}'?",
    "translate '{w}' into Spanish", "a synonym for '{w}'?", "define '{w}' briefly",
    "what day is it?", "what's the time zone of {country}?", "say hi in French",
    "list three {thing}s", "is the {thing} blue?", "yes or no: is {n1} prime?",
    "convert {n1} to text", "what's the first letter of '{w}'?",
    "give me a one word answer: {w}", "how do you spell {country}?",
]
EASY_PREFIX = ["", "", "hey, ", "quick one: ", "simple q: ", "just curious, ", "btw "]

MEDIUM = [
    "Write a Python function to {task}.", "Explain how {concept} works in simple terms.",
    "Summarize the main idea of {concept} in a paragraph.",
    "Write a regex that matches {pattern}.",
    "Convert this list of {thing}s into a CSV with headers.",
    "What are the pros and cons of {concept}?",
    "Write a SQL query to find the top 5 {thing}s by count.",
    "Refactor this small function to be more readable.",
    "Explain the difference between {concept} and {concept2}.",
    "Write unit tests for a function that adds two numbers.",
    "Give me a step-by-step plan to learn {concept}.",
    "Parse this JSON and extract the {thing} field.",
    "Write a bash script to rename files by extension.",
    "Describe how to set up {concept} for a small project.",
    "Turn this paragraph into three bullet points.",
    "What is the time complexity of bubble sort and why?",
    "Write a function to validate an email address.",
    "Explain {concept} to a junior engineer with one example.",
    "Outline the steps to deploy a static site.",
    "Generate a small example config for {concept}.",
]
MEDIUM_PREFIX = ["", "", "Could you ", "Please ", "I need help: ", "Help me ", "Can you "]

HARD = [
    "Prove that there are infinitely many primes of the form 4k+3.",
    "Design a horizontally scalable, multi-region rate limiter and analyze the tradeoffs.",
    "Refactor this {n1}-line module to remove the circular dependency and add property-based tests.",
    "Derive the gradient of softmax cross-entropy and explain numerical stability.",
    "Debug this deadlock in a concurrent producer-consumer system and prove your fix is correct.",
    "Design a consensus protocol tolerant to {n1} Byzantine failures and prove its safety and liveness.",
    "Explain the CAP theorem implications for a globally distributed datastore with concrete examples.",
    "Implement a lock-free MPSC queue in Rust and argue its memory ordering is sound.",
    "Analyze the asymptotic complexity of this recursive algorithm and optimize it with a proof.",
    "Architect a zero-downtime migration for a {n1}-shard database with a rollback plan.",
    "Formally verify that this state machine cannot reach the error state.",
    "Synthesize a literature review on {concept} retrieval and identify the open research problems.",
    "Optimize this query plan; explain why the chosen index and join order are optimal.",
    "Reason rigorously about the stability of this numerical integration scheme.",
    "Prove the correctness of Dijkstra's algorithm for non-negative edge weights.",
    "Design an exactly-once delivery pipeline across services under partial failure and justify it.",
    "Extend this type system with {concept} and argue soundness and decidability.",
    "Given these conflicting non-functional requirements, design the system and defend every tradeoff.",
    "Develop and prove an invariant that guarantees this distributed lock is mutually exclusive.",
    "Construct a reduction showing this scheduling problem is NP-hard.",
]
HARD_PREFIX = ["", "", "I'm stuck: ", "For a design review, ", "Rigorously: ", "In depth, "]

WORDS = ["river", "matrix", "kernel", "photon", "ledger", "quartz", "cobalt", "syntax",
         "vector", "ember", "harbor", "lantern", "cipher", "meadow", "glacier"]
DAYS = ["Monday", "Tuesday", "Wednesday", "Friday", "Sunday"]
COUNTRIES = ["France", "Japan", "Brazil", "Egypt", "Canada", "Norway", "Kenya", "Peru"]
THINGS = ["sky", "grass", "user", "order", "invoice", "sensor", "node", "file", "row"]
TASKS = ["reverse a string", "count word frequencies", "merge two sorted lists",
         "flatten a nested list", "compute a moving average", "deduplicate a list",
         "check if a string is a palindrome", "parse a date string"]
CONCEPTS = ["OAuth2", "Docker", "indexing", "caching", "REST", "gRPC", "pub/sub",
            "vectorization", "garbage collection", "load balancing", "hashing",
            "pagination", "memoization", "transactions", "sharding"]
PATTERNS = ["a US phone number", "a hex color", "an ISO date", "a slug",
            "an IPv4 address", "a semantic version"]


def fill(t):
    return t.format(
        n1=random.randint(2, 990), n2=random.randint(2, 99),
        w=random.choice(WORDS), day=random.choice(DAYS),
        country=random.choice(COUNTRIES), thing=random.choice(THINGS),
        task=random.choice(TASKS), concept=random.choice(CONCEPTS),
        concept2=random.choice(CONCEPTS), pattern=random.choice(PATTERNS),
    )


def gen(templates, prefixes, label, n, rng):
    out, seen = [], set()
    tries = 0
    while len(out) < n and tries < n * 50:
        tries += 1
        text = (random.choice(prefixes) + fill(random.choice(templates))).strip()
        if text and text not in seen:
            seen.add(text)
            out.append({"text": text, "label": label})
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n-per-class", type=int, default=3000)
    ap.add_argument("--seed", type=int, default=13)
    ap.add_argument("--out", default="-")
    args = ap.parse_args()
    random.seed(args.seed)

    rows = []
    rows += gen(EASY, EASY_PREFIX, 0, args.n_per_class, random)
    rows += gen(MEDIUM, MEDIUM_PREFIX, 1, args.n_per_class, random)
    rows += gen(HARD, HARD_PREFIX, 2, args.n_per_class, random)
    random.shuffle(rows)

    f = open(args.out, "w") if args.out != "-" else __import__("sys").stdout
    for r in rows:
        f.write(json.dumps(r) + "\n")
    if args.out != "-":
        f.close()
        counts = {0: 0, 1: 0, 2: 0}
        for r in rows:
            counts[r["label"]] += 1
        print(f"wrote {len(rows)} rows to {args.out}  (easy={counts[0]} medium={counts[1]} hard={counts[2]})")


if __name__ == "__main__":
    main()
