# forge-cli

The `forge` command-line tool for the **Travel Innovation Platform** -- chat with LLMs, manage models and configuration, run agents, scaffold containerized agent projects, and start multi-port servers, all from the terminal.

> Part of the [liteforge](../../README.md) workspace. Powered by the [`liteforge`](../liteforge/) Rust core.

## Installation

**macOS / Linux:**
```bash
git clone https://gitea.poyner.ai/sean/liteforge.git /tmp/liteforge && bash /tmp/liteforge/scripts/install.sh && rm -rf /tmp/liteforge
```

**Windows (PowerShell):**
```powershell
git clone https://gitea.poyner.ai/sean/liteforge.git $env:TEMP\liteforge; & $env:TEMP\liteforge\scripts\install.ps1; Remove-Item -Recurse -Force $env:TEMP\liteforge
```

**Homebrew:**
```bash
brew tap sean/forge https://gitea.poyner.ai/sean/homebrew-forge.git
brew install forge-cli
```

**From source:**
```bash
cargo install --path crates/forge-cli
```

## Commands

| Command | Description |
|---------|-------------|
| `forge chat` | Chat with an LLM (streaming, interactive REPL, system prompts, piped input) |
| `forge models` | List and inspect available models with provider detection |
| `forge config` | View/set configuration, manage paths, and store keyring secrets |
| `forge embed` | Generate vector embeddings from text or files |
| `forge chunk` | Split text into chunks for RAG pipelines |
| `forge agents` | List, inspect, and run YAML-defined agents |
| `forge tools` | List tools available to agents |
| `forge claude` | Launch Claude Code with LiteForge env vars, MCP config, and usage tracking |
| `forge usage` | View API usage reports (monthly, weekly, by-model, sessions) |
| `forge guardrails` | Check text for PII and prompt injection |
| `forge mcp` | Manage MCP server configurations |
| `forge serve` | Start multi-port agent server (user, MCP, tools, A2A, knowledge, skills) |
| `forge infra` | Manage observability infrastructure (Docker-based) |
| `forge adk` | Agent Development Kit -- scaffold, build, run, and test containerized agents |

## Key Workflows

### Chat

```bash
forge chat "What is the capital of France?"
forge chat --stream "Tell me a story"
forge chat --system "You are a pirate" "Where is the treasure?"
forge chat --model gpt-4o "Explain quantum computing"
forge chat -i                              # interactive REPL
echo "Summarize this" | forge chat         # piped input
```

### Agent Development Kit (ADK)

The ADK provides a full lifecycle for building containerized agent ecosystems:

```bash
forge adk init my-agent     # 1. Scaffold project
forge adk dev               # 2. Hot-reload dev server
forge adk validate          # 3. Validate configuration
forge adk test              # 4. Run eval suite
forge adk build             # 5. Build container image
forge adk run               # 6. Run container
forge adk run -D            #    ...detached mode
forge adk logs -f           # 7. Follow logs
forge adk stop              # 8. Stop container
```

**Scaffolded project structure:**

```
my-agent/
├── adk.yaml              # Project manifest
├── .env.example          # Environment template
├── agents/
│   └── example.yaml      # Agent YAML definitions
├── tools/
│   └── example_tool.py   # Python tool implementations
├── knowledge/
│   └── docs/             # Knowledge base documents
├── skills/               # Custom skill definitions
└── tests/
    └── test_example.yaml # Eval test cases
```

### Multi-Port Server (`forge serve`)

Start role-specific HTTP servers individually or all together:

```bash
forge serve                        # all servers
forge serve user                   # user-facing API only
forge serve mcp                    # MCP protocol server only
forge serve --user-port 9000       # override port
forge serve --config ./serve.toml  # custom config file
```

**Default ports:**

| Role | Port | Description |
|------|:----:|-------------|
| User | 8080 | User-facing chat/completions API |
| MCP | 8081 | Model Context Protocol server |
| Tools | 8082 | Tools REST API |
| A2A | 8083 | Agent-to-Agent communication |
| Knowledge | 8084 | Knowledge base REST API |
| Skills | 8085 | Skills REST API |

Configure via `~/.config/forge/serve.toml`:

```toml
[user]
host = "127.0.0.1"
port = 8080
enabled = true

[mcp]
port = 8081
enabled = true

[tools]
port = 8082
enabled = true

[a2a]
port = 8083
enabled = true

[knowledge]
port = 8084
enabled = true

[skills]
port = 8085
enabled = true
```

### Configuration & Secrets

```bash
forge config show                   # display all settings
forge config set api-key sk-xxxx    # set env value (writes to .env)
forge config set agents-dir ./mine  # set path override (writes to config.toml)
forge config get model              # get a single value
forge config init                   # initialize config directory structure
forge config paths                  # show all effective paths

# Keyring secret management
forge config set-secret forge-api-key       # store (prompts interactively)
forge config get-secret forge-api-key       # retrieve (stdout, for scripting)
forge config list-secrets                 # show status of all secrets
forge config delete-secret forge-api-key    # remove
```

Valid secret keys: `forge-api-key`, `artifactory-user`, `artifactory-key`.

Secrets are stored in the platform's native credential manager as a secondary mechanism. The primary storage is `~/.forge/config.toml` and shell env files:

| Platform | Secondary Backend |
|----------|---------|
| macOS | Keychain |
| Linux | Secret Service (GNOME Keyring / KWallet) |
| Windows | Credential Manager |

### Claude Code Integration

`forge claude` launches Claude Code with LiteForge environment variables and MCP servers pre-configured. A local proxy automatically strips `context_management` fields and `context-management-*` beta headers that LiteForge's LiteLLM rejects, so Claude Code works unmodified.

```bash
forge claude                  # launch with LiteForge config + MCP servers
forge claude -p "write tests" # pass arguments through to claude
forge claude --print-env      # show env vars that would be set
forge claude --print-mcp      # show MCP server configuration
forge claude --no-track       # disable usage tracking
forge claude --api-key sk-xx  # override API key
forge claude --base-url URL   # override base URL
```

Default model mapping (override via environment variables):

| Variable | Default |
|---|---|
| `ANTHROPIC_MODEL` | `anthropic.claude-opus-4-5-20251101-v1:0` |
| `ANTHROPIC_SMALL_FAST_MODEL` | `anthropic.claude-haiku-4-5-20251001-v1:0` |

### Usage Tracking

```bash
forge usage                   # monthly summary
forge usage --period weekly   # weekly summary
forge usage --by-model        # breakdown by model
forge usage --sessions        # list sessions
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LITEFORGE_API_KEY` | API key for authentication | Required |
| `LITEFORGE_BASE_URL` | LiteLLM endpoint URL | LiteForge production endpoint |
| `LITEFORGE_DEFAULT_MODEL` | Default model | `anthropic.claude-haiku-4-5-20251001-v1:0` |
| `LITEFORGE_TIMEOUT` | Request timeout in seconds | `60` |

### Config File

Located at `~/.config/forge/config.toml` (or platform equivalent):

```toml
api_key = "your-api-key"

[endpoints]
base_url = "https://api.example.com/v1"

[defaults]
model = "anthropic.claude-haiku-4-5-20251001-v1:0"
timeout = 60

[paths]
# agents_dir = "~/.config/forge/agents"
# skills_dir = "~/.config/forge/skills"
# mcp_dir = "~/.config/forge/mcp"
# tools_dir = "~/.config/forge/tools"
```

## Theming

The CLI uses a **Dracula** color palette with a **brand accent** brand accent. It auto-detects terminal capabilities and falls back gracefully:

- **Truecolor** (24-bit) -- full Dracula palette
- **256-color** -- approximate colors
- **Basic ANSI** -- standard 16 colors
- **No color** -- plain text (respects `NO_COLOR` env var)

The help banner displays an ASCII art Forge wordmark styled with the active palette.

## Building from Source

Building from source requires a Rust toolchain (1.70+) and a C compiler/linker. On Linux (Debian/Ubuntu):

```bash
sudo apt-get install -y build-essential pkg-config libssl-dev
```

Then build and install:

```bash
cargo install --path crates/forge-cli
forge --version
```

Or build without installing:

```bash
cargo build --release -p forge-cli
./target/release/forge --help
```

## Full Documentation

See the [CLI Reference](../../docs/cli.md) and the [MkDocs site](https://seanpoyner.github.io/liteforge/) for detailed command documentation.
