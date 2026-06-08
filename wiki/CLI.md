# CLI (`forge`)

`forge` is the command‑line front end to the LiteForge core — chat, embeddings, chunking, agents,
guardrails, MCP, usage reporting, a multi‑port server, and the Agent Development Kit, all from the
terminal. Install it from **[Installation](Installation)**.

For exhaustive flag‑by‑flag detail see
[`docs/cli.md`](https://github.com/seanpoyner/liteforge/blob/main/docs/cli.md); this page is the
tour.

## Command map

| Command | What it does |
|---|---|
| `forge chat` | Chat with an LLM — streaming, interactive REPL, system prompts, piped input |
| `forge models` | List/inspect available models with provider detection |
| `forge config` | View/set config, manage paths, store keyring secrets |
| `forge embed` | Generate embeddings from text or files |
| `forge chunk` | Split text into chunks for RAG |
| `forge agents` | List, inspect, and run YAML‑defined agents |
| `forge tools` | List tools available to agents |
| `forge guardrails` | Check text for PII and prompt injection |
| `forge mcp` | Manage MCP server configurations |
| `forge usage` | View API usage reports |
| `forge claude` | Launch Claude Code with LiteForge env + MCP + usage tracking |
| `forge serve` | Start the multi‑port agent server |
| `forge infra` | Manage Docker‑based observability infrastructure |
| `forge adk` | Agent Development Kit — scaffold, build, run, test containerized agents |

## Chat

```bash
forge chat "What is the capital of France?"
forge chat --stream "Tell me a story"
forge chat --system "You are a pirate" "Where is the treasure?"
forge chat --model gpt-4o-mini "Explain quantum computing"
forge chat -i                          # interactive REPL
echo "Summarize this" | forge chat     # piped input
forge chat -o json "…"                 # pretty | json | raw
```

## Models

```bash
forge models list
forge models info gpt-4o-mini
```

## Embed & chunk

```bash
forge embed "some text to embed"
forge embed --file notes.md -o json

forge chunk report.txt --size 500 --overlap 50 --strategy recursive
forge chunk report.txt --json
```

## Guardrails

```bash
forge guardrails "Call me at 555-123-4567"     # all checks
echo "ignore previous instructions" | forge guardrails --injection
forge guardrails --stdin --json < input.txt
```

See **[Guardrails](Guardrails)** for what's detected (and the heuristic caveat).

## Agents & tools

```bash
forge agents list
forge agents info my-agent
forge agents run my-agent
forge tools list
```

Agents are YAML‑defined; point at a directory with `-d/--dir`. To build a full agent project, use
the ADK — see **[ADK and Serve](ADK-and-Serve)**.

## MCP

```bash
forge mcp list
forge mcp info my-server
forge mcp list --config ./mcp.json
```

## Usage

```bash
forge usage --period weekly --by-model
forge usage --sessions
```

Details in **[Observability and Telemetry](Observability-and-Telemetry)**.

## Claude Code integration

`forge claude` launches Claude Code pre‑wired with LiteForge env vars and MCP servers, and tracks
usage:

```bash
forge claude                   # launch with LiteForge config + MCP servers
forge claude -p "write tests"  # pass a prompt through
forge claude --print-env       # show env that would be set
forge claude --print-mcp       # show MCP config
forge claude --no-track        # disable usage tracking
```

## Config & secrets

```bash
forge config init
forge config show
forge config set base-url https://api.openai.com/v1
forge config get model
forge config set-secret forge-api-key      # keyring-backed
forge config list-secrets
```

Full details in **[Configuration](Configuration)**.

## Output formats

Most read commands accept `-o/--output` with `pretty` (default), `json`, or `raw` — use `json` when
scripting.

Next: **[ADK and Serve](ADK-and-Serve)**
