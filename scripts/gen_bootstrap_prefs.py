#!/usr/bin/env python3
"""Generate a small difficulty-labeled preference dataset to bootstrap the MF
router before full RouteLLM Arena data is wired in.

Produces JSONL of {"prompt": ..., "label": 0|1} where label=1 means "the strong
model is needed" (hard reasoning/coding/math/system-design) and 0 means "a weak
model suffices" (greetings, simple facts, formatting, basic arithmetic).

    python scripts/gen_bootstrap_prefs.py > prefs.jsonl
    python scripts/retrain_mf.py --data prefs.jsonl --embedding-model bge-m3 \
        --dimensions 1024 --out mf_weights.json

For production, replace this with RouteLLM's released Arena preference data
(lm-sys/RouteLLM): for a chosen strong/weak anchor pair, label=1 when the strong
model wins the battle. The training script accepts any such JSONL.
"""
import json
import random
import sys

random.seed(7)

EASY = [
    "hi", "hello there", "thanks!", "good morning", "how are you?",
    "what's 2+2?", "what is {n1} plus {n2}?", "spell the word '{w}'",
    "what day comes after {day}?", "translate '{w}' to French",
    "give me a synonym for '{w}'", "what color is the {thing}?",
    "convert {n1} to a string", "uppercase the word '{w}'",
    "what's the capital of {country}?", "is {n1} an even number?",
    "summarize this in one word: {w}", "reverse the word '{w}'",
    "what's the weather like today?", "tell me a short greeting",
    "round {n1}.5 to the nearest integer", "what time zone is {country} in?",
    "define the word '{w}'", "say hello in Spanish",
    "what is the plural of '{w}'?", "list three colors",
]
HARD = [
    "Prove that there are infinitely many primes of the form 4k+3.",
    "Refactor this {n1}-line module to remove the circular dependency and add tests.",
    "Design a horizontally scalable rate limiter for a multi-region API and discuss tradeoffs.",
    "Derive the gradient of softmax cross-entropy and explain numerical stability.",
    "Debug this deadlock in a concurrent producer-consumer system and propose a fix.",
    "Prove the {country} theorem rigorously using induction and explain each step.",
    "Explain the CAP theorem implications for a globally distributed datastore with examples.",
    "Write a correct lock-free queue in Rust and argue its memory ordering is sound.",
    "Analyze the time and space complexity of this recursive algorithm and optimize it.",
    "Design a consensus protocol tolerant to {n1} byzantine failures and prove safety.",
    "Given these conflicting requirements, architect a migration plan with rollback.",
    "Formally verify that this state machine cannot reach the error state.",
    "Synthesize a literature review on {w} retrieval and identify open problems.",
    "Optimize this SQL query plan and explain why the index choice matters.",
    "Reason step by step about the {n1}-body problem and its chaotic behavior.",
    "Prove correctness of Dijkstra's algorithm with non-negative weights.",
    "Explain how to make this distributed transaction exactly-once under failures.",
    "Design a type system extension for {w} and discuss soundness.",
]
WORDS = ["river", "matrix", "kernel", "photon", "ledger", "quartz", "cobalt", "syntax", "vector", "ember"]
DAYS = ["Monday", "Tuesday", "Friday", "Sunday"]
COUNTRIES = ["France", "Japan", "Brazil", "Egypt", "Canada", "Pythagoras", "binomial"]
THINGS = ["sky", "grass", "sun", "ocean"]


def fill(t):
    return t.format(
        n1=random.randint(2, 900), n2=random.randint(2, 99),
        w=random.choice(WORDS), day=random.choice(DAYS),
        country=random.choice(COUNTRIES), thing=random.choice(THINGS),
    )


def main():
    rows = []
    for t in EASY:
        for _ in range(7):
            rows.append({"prompt": fill(t), "label": 0})
    for t in HARD:
        for _ in range(9):
            rows.append({"prompt": fill(t), "label": 1})
    random.shuffle(rows)
    seen, uniq = set(), []
    for r in rows:
        if r["prompt"] not in seen:
            seen.add(r["prompt"])
            uniq.append(r)
    for r in uniq:
        sys.stdout.write(json.dumps(r) + "\n")


if __name__ == "__main__":
    main()
