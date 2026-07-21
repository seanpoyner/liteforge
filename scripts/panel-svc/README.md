# router-panel service

A multi-signal routing classifier: four independent tiny BERT experts (task type,
difficulty, reasoning depth, context demand) plus structured codebase-context
features, combined by a learned fusion matrix into capability-group scores
(chat / code / reasoning / long_context / general). Served OpenAI-compatibly for
LiteForge's `remote_classifier` selector.

## Train the experts + fusion

```bash
python ../gen_panel_data.py        --n 14000          --out panel.jsonl
python ../train_panel_experts.py   --data panel.jsonl --out .            # writes task_type/ etc.
python ../train_panel_fusion.py    --data panel.jsonl --out fusion.json
```

Each expert is ~4.4M params (bert-tiny); all four plus the fusion matrix reach
~100% in-distribution held-out accuracy on the synthetic data. Use real labeled
traffic for production-meaningful numbers.

## Serve

```bash
docker build -t router-panel:local .
docker run -d --name router-panel --restart unless-stopped -p 8078:8078 router-panel:local
curl localhost:8078/v1/chat/completions -H 'content-type: application/json' \
  -d '{"messages":[{"role":"user","content":"refactor this across 5 files"}]}'
# -> content: {"scores":{...}, "signals":{task_type,difficulty,reasoning_depth,context_demand}, "features":{...}}
```

## Wire into LiteForge

```yaml
selector:
  kind: remote_classifier
  endpoint: { kind: chat, model: router-panel, forward_messages: true }  # forwards full context
  label_to_group: { chat: chat, code: code, reasoning: reasoning, long_context: long_context, general: general }
```
