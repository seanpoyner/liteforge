"""OpenAI-compatible serving shim for the panel router.

POST /v1/chat/completions: concatenates the message contents (so codebase context
is visible), runs the 4 tiny BERT experts + structured features + fusion matrix,
and returns capability-group scores as the message content:
    {"scores": {"chat":..,"code":..,"reasoning":..,"long_context":..,"general":..},
     "signals": {"task_type":..,"difficulty":..,"reasoning_depth":..,"context_demand":..}}
LiteForge's remote_classifier selector reads `scores`; `signals` is for observability.
"""
import json
import time

from fastapi import FastAPI
from pydantic import BaseModel

from panel_infer import Panel

panel = Panel("/app")
app = FastAPI(title="router-panel")


class Message(BaseModel):
    role: str
    content: str | None = None


class ChatRequest(BaseModel):
    model: str | None = None
    messages: list[Message] = []


@app.get("/health")
def health():
    return {"status": "ok", "groups": panel.groups, "signals": list(panel.classes.keys())}


@app.post("/v1/chat/completions")
def chat_completions(req: ChatRequest):
    # Concatenate all non-empty message contents so the panel sees the full
    # prompt + codebase context, not just the last user turn.
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
