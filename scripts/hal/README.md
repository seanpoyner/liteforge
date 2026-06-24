# hal-9000 routing infra helpers

Keeps the routing embedding model (`bge-m3`) resident in Ollama so routing latency
stays low (a cold load costs 5-8s; resident is ~0.2s).

```bash
sudo cp warm-bge.sh /usr/local/bin/ && sudo chmod +x /usr/local/bin/warm-bge.sh
sudo cp ollama-bge-warm.{service,timer} /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now ollama-bge-warm.timer
```

Also point the router's embedding endpoint at the WG-direct LiteLLM
(`http://10.8.0.6:4000/v1`) rather than the public `litellm.poyner.ai` tunnel: the
tunnel adds 0.5-6s of variable latency; WG-direct is a steady ~0.3s.
