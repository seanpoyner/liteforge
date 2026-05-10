#!/bin/bash
set -e

RUST_CLI="forge"
PYTHON_CLI="liteforge"
RESULTS_DIR="benchmarks/results"
PROJECT_DIR="/mnt/c/users/sbpoy839/.projects/liteforge"

cd "$PROJECT_DIR"
mkdir -p "$RESULTS_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUTFILE="$RESULTS_DIR/benchmark_$TIMESTAMP.txt"

echo "Speed Test: Forge CLI benchmarks" | tee "$OUTFILE"
echo "========================================" | tee -a "$OUTFILE"
echo "Date: $(date)" | tee -a "$OUTFILE"
echo "Rust CLI: $(which $RUST_CLI)" | tee -a "$OUTFILE"
echo "Python CLI: $(which $PYTHON_CLI)" | tee -a "$OUTFILE"
echo "" | tee -a "$OUTFILE"

echo ""
echo "### 1. CLI Startup: --help ###" | tee -a "$OUTFILE"
hyperfine --warmup 3 --runs 20 \
    --export-markdown "$RESULTS_DIR/help.md" \
    "$RUST_CLI --help" "$PYTHON_CLI --help" 2>&1 | tee -a "$OUTFILE"

echo ""
echo "### 2. CLI Startup: --version ###" | tee -a "$OUTFILE"
hyperfine --warmup 3 --runs 20 \
    --export-markdown "$RESULTS_DIR/version.md" \
    "$RUST_CLI --version" "$PYTHON_CLI --version" 2>&1 | tee -a "$OUTFILE"

echo ""
echo "### 3. Config Loading ###" | tee -a "$OUTFILE"
hyperfine --warmup 3 --runs 20 \
    --export-markdown "$RESULTS_DIR/config.md" \
    "$RUST_CLI config show" "$PYTHON_CLI config" 2>&1 | tee -a "$OUTFILE"

echo ""
echo "### 4. Text Chunking (fixed strategy) ###" | tee -a "$OUTFILE"
hyperfine --warmup 3 --runs 20 \
    --export-markdown "$RESULTS_DIR/chunk_fixed.md" \
    "$RUST_CLI chunk README.md" "$PYTHON_CLI chunk README.md" 2>&1 | tee -a "$OUTFILE"

echo ""
echo "### 5. Text Chunking (sentence strategy) ###" | tee -a "$OUTFILE"
hyperfine --warmup 3 --runs 20 \
    --export-markdown "$RESULTS_DIR/chunk_sentence.md" \
    "$RUST_CLI chunk README.md --strategy sentence" "$PYTHON_CLI chunk README.md --strategy sentence" 2>&1 | tee -a "$OUTFILE"

echo ""
echo "### 6. Guardrails PII Check ###" | tee -a "$OUTFILE"
hyperfine --warmup 3 --runs 20 \
    --export-markdown "$RESULTS_DIR/guardrails.md" \
    "$RUST_CLI guardrails check 'Contact me at test@example.com or 555-123-4567' --pii" \
    "$PYTHON_CLI guardrails check 'Contact me at test@example.com or 555-123-4567' --pii" 2>&1 | tee -a "$OUTFILE"

echo ""
echo "========================================" | tee -a "$OUTFILE"
echo "Results saved to: $OUTFILE" | tee -a "$OUTFILE"
echo ""
echo "Individual markdown results in: $RESULTS_DIR/"
