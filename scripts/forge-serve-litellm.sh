#!/usr/bin/env bash
# Start `forge serve user` pointed at a LiteLLM (or any OpenAI-compatible) gateway.
#
# Required env:
#   LITELLM_API_KEY   The bearer token the gateway expects (your LiteLLM master/user key).
#
# Optional env:
#   LITELLM_BASE_URL  Defaults to https://your-gateway.example.com/v1
#   LITEFORGE_DEFAULT_MODEL  Sent on requests that don't specify a model.
#                            Defaults to anthropic.claude-haiku-4-5-20251001-v1:0
#                            (matches the model id seen in the demo notebook).
#   USER_PORT         Defaults to 8080.
#
# Usage:
#   LITELLM_API_KEY=sk-xxx scripts/forge-serve-litellm.sh
#   # or
#   export LITELLM_API_KEY=sk-xxx
#   scripts/forge-serve-litellm.sh

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

# Auto-load .env from the repo root if present, so the user only edits one file.
if [[ -f "$REPO_ROOT/.env" ]]; then
    set -a
    # shellcheck disable=SC1091
    source "$REPO_ROOT/.env"
    set +a
fi

if [[ -z "${LITELLM_API_KEY:-}" ]]; then
    echo "error: LITELLM_API_KEY is not set." >&2
    echo "       Set it in $REPO_ROOT/.env or export it in your shell, then re-run." >&2
    exit 2
fi

LITELLM_BASE_URL="${LITELLM_BASE_URL:-https://your-gateway.example.com/v1}"
USER_PORT="${USER_PORT:-8080}"
DEFAULT_MODEL="${LITEFORGE_DEFAULT_MODEL:-anthropic.claude-haiku-4-5-20251001-v1:0}"

FORGE="$REPO_ROOT/target/release/forge"
if [[ ! -x "$FORGE" ]]; then
    echo "building forge-cli (release)..." >&2
    (cd "$REPO_ROOT" && cargo build --release -p forge-cli) >&2
fi

echo "starting forge serve user"
echo "  upstream: $LITELLM_BASE_URL"
echo "  listen:   http://127.0.0.1:$USER_PORT"
echo "  model:    $DEFAULT_MODEL"

exec env \
    LITEFORGE_BASE_URL="$LITELLM_BASE_URL" \
    LITEFORGE_API_KEY="$LITELLM_API_KEY" \
    LITEFORGE_DEFAULT_MODEL="$DEFAULT_MODEL" \
    "$FORGE" serve --user-port "$USER_PORT" user
