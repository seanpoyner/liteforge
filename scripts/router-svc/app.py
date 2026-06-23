"""OpenAI-compatible serving shim for the BERT-style routing classifier.

Exposes POST /v1/chat/completions: it reads the last user message, runs the
fine-tuned 3-class encoder (easy/medium/hard), and returns a normal chat
completion whose message content is the JSON `{"scores": {label: prob, ...}}`.
This lets LiteLLM register it as an `openai/`-style model and lets LiteForge's
RemoteClassifierSelector (chat mode) parse the scores. Tiny model -> CPU is instant.
"""
import json
import time

import torch
import torch.nn.functional as F
from fastapi import FastAPI
from pydantic import BaseModel
from transformers import AutoModelForSequenceClassification, AutoTokenizer

MODEL_DIR = "/app/model"
LABELS = ["easy", "medium", "hard"]

tok = AutoTokenizer.from_pretrained(MODEL_DIR)
model = AutoModelForSequenceClassification.from_pretrained(MODEL_DIR)
model.eval()

app = FastAPI(title="router-bert classifier")


class Message(BaseModel):
    role: str
    content: str | None = None


class ChatRequest(BaseModel):
    model: str | None = None
    messages: list[Message] = []


@torch.no_grad()
def classify(text: str) -> dict:
    enc = tok(text, truncation=True, padding=True, max_length=64, return_tensors="pt")
    logits = model(**enc).logits
    probs = F.softmax(logits, dim=-1)[0].tolist()
    return {LABELS[i]: round(float(p), 6) for i, p in enumerate(probs)}


@app.get("/health")
def health():
    return {"status": "ok", "labels": LABELS}


@app.post("/v1/chat/completions")
def chat_completions(req: ChatRequest):
    text = ""
    for m in reversed(req.messages):
        if m.role == "user" and m.content:
            text = m.content
            break
    scores = classify(text)
    content = json.dumps({"scores": scores})
    return {
        "id": "router-bert-1",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": req.model or "router-bert",
        "choices": [
            {
                "index": 0,
                "message": {"role": "assistant", "content": content},
                "finish_reason": "stop",
            }
        ],
        "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0},
    }
