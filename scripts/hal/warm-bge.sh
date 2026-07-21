#!/bin/sh
# Keep bge-m3 resident in Ollama for low-latency routing embeddings.
# Deployed to /usr/local/bin/warm-bge.sh on hal-9000, run by ollama-bge-warm.timer.
curl -s -o /dev/null -m 60 http://localhost:11434/api/embed \
  -H "Content-Type: application/json" \
  -d '{"model":"bge-m3","input":"warm","keep_alive":"1h"}'
