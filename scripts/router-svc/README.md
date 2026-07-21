# router-bert classifier service

OpenAI-compatible serving shim for the fine-tuned BERT-style routing classifier,
used by LiteForge's `remote_classifier` model selector.

## Build the model

```bash
python ../gen_router_clf_data.py --n-per-class 3000 --out clf.jsonl
python ../train_router_clf.py --data clf.jsonl --model prajjwal1/bert-tiny --out model
```

`prajjwal1/bert-tiny` (~4.4M params) reaches ~99.9% in-distribution held-out
accuracy on the synthetic 3-class (easy/medium/hard) data. Use a real labeled
corpus for production-meaningful accuracy.

## Serve

```bash
docker build -t router-bert:local .
docker run -d --name router-bert --restart unless-stopped -p 8077:8077 router-bert:local
curl localhost:8077/v1/chat/completions -H 'content-type: application/json' \
  -d '{"messages":[{"role":"user","content":"prove this theorem"}]}'
# -> {"choices":[{"message":{"content":"{\"scores\": {\"easy\":..,\"medium\":..,\"hard\":..}}"}}]}
```

## Wire into LiteLLM + LiteForge

LiteLLM `model_list`:

```yaml
- model_name: router-bert
  litellm_params:
    model: openai/router-bert
    api_base: http://host.docker.internal:8077/v1
    api_key: sk-noauth
```

LiteForge router `model_routing.selector`:

```yaml
kind: remote_classifier
endpoint: { kind: chat, model: router-bert }
label_to_group: { easy: cheap, medium: balanced, hard: premium }
```
