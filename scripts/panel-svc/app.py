"""OpenAI-compatible serving shim for the router-panel.

POST /v1/chat/completions: concatenates the message contents (so codebase context is
visible), embeds via bge-m3, runs the learned quality + task heads + structured
features, and returns capability-group scores as the message content:
    {"scores": {"chat":..,"code":..,"reasoning":..,"long_context":..,"general":..},
     "signals": {"hardness":.., "task":..}, "features": {...}}
LiteForge's remote_classifier selector reads `scores`; the rest is observability.
"""
import json
import time

from fastapi import FastAPI
from pydantic import BaseModel

from embedding_infer import EmbeddingHead

panel = EmbeddingHead()
app = FastAPI(title="router-panel")


class Message(BaseModel):
    role: str
    content: str | None = None


class ChatRequest(BaseModel):
    model: str | None = None
    messages: list[Message] = []


@app.get("/health")
def health():
    return {"status": "ok", "groups": panel.spec.get("groups", []),
            "embedding_model": panel.spec.get("embedding_model")}


@app.post("/v1/chat/completions")
def chat_completions(req: ChatRequest):
    parts = [m.content for m in req.messages if m.content]
    text = "\n\n".join(parts) if parts else ""
    scores, signals, feats = panel.classify(text)
    content = json.dumps({"scores": scores, "signals": signals, "features": feats})
    return {
        "id": "router-panel-1",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": req.model or "router-panel",
        "choices": [{"index": 0, "message": {"role": "assistant", "content": content},
                     "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0},
    }
