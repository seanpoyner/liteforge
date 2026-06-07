# CLI Reference

The `forge` command-line tool provides access to LiteForge features from the terminal.

## Installation

=== "Quick Install (macOS / Linux)"

    ```bash
    curl -fsSL https://raw.githubusercontent.com/seanpoyner/liteforge/main/scripts/install.sh | bash
    ```

=== "Quick Install (Windows)"

    ```powershell
    irm https://raw.githubusercontent.com/seanpoyner/liteforge/main/scripts/install.ps1 | iex
    ```

=== "Homebrew"

    ```bash
    brew tap seanpoyner/forge https://github.com/seanpoyner/homebrew-forge.git
    brew install forge-cli
    ```

=== "From Source"

    ```bash
    cargo install --path crates/forge-cli
    ```

See the [Installation Guide](installation.md) for full details.

## Commands

### `forge chat`

Interactive chat with an LLM.

```bash
# Single message
forge chat "What is Rust?"

# Streaming output (default)
forge chat --stream "Tell me a story"

# Interactive REPL
forge chat -i

# With system prompt
forge chat --system "You are a pirate" "Where is the treasure?"

# With model selection
forge chat --model gpt-4o "Explain quantum computing"

# Piped input
echo "Summarize this" | forge chat
```

### `forge models`

List and inspect available models with provider detection and capability info.

```bash
# List all models
forge models list

# Show details for a specific model
forge models info gpt-4o

# Output as JSON
forge models list --output json
```

### `forge config`

View and update SDK configuration, manage paths, and store secrets in the platform keyring.

```bash
# Show all settings (env + paths)
forge config show

# Set environment values (written to .env)
forge config set api-key sk-xxxx
forge config set model gpt-4o

# Set path overrides (written to config.toml)
forge config set agents-dir ./my-agents

# Get a value
forge config get model

# Initialize config directory structure
forge config init

# Show all effective paths
forge config paths
```

#### Secret Management

Secrets are stored in the platform's native credential manager (macOS Keychain, Windows Credential Manager, or Linux Secret Service).

```bash
# Store a secret (prompts interactively if value omitted)
forge config set-secret forge-api-key

# Retrieve a secret (outputs to stdout for scripting)
forge config get-secret forge-api-key

# List all secrets and their status
forge config list-secrets

# Delete a secret
forge config delete-secret forge-api-key
```

Valid secret keys: `forge-api-key`, `artifactory-user`, `artifactory-key`

### `forge embed`

Create text embeddings.

```bash
forge embed "Hello world"
forge embed --model text-embedding-3-small "Hello"
forge embed --file document.txt
echo "text" | forge embed
```

### `forge chunk`

Chunk text for RAG pipelines.

```bash
# From file with options
forge chunk document.txt --size 1000 --overlap 100

# Sentence-based strategy with JSON output
forge chunk document.txt --strategy sentence --json

# From string
forge chunk --size 512 --overlap 50 --strategy recursive "Your text..."
```

### `forge agents`

Manage and run agents from YAML configuration files.

```bash
# List configured agents
forge agents list

# Inspect an agent
forge agents info my-agent

# Run an agent
forge agents run my-agent
```

### `forge tools`

List available tools for agents.

```bash
# List all tools
forge tools list

# List tools for a specific agent
forge tools list --agent my-agent
```

### `forge claude`

Launch Claude Code with LiteForge configuration and MCP servers pre-configured. A local proxy automatically strips `context_management` fields and `context-management-*` beta headers that LiteForge's LiteLLM rejects, so Claude Code works unmodified. Optionally tracks usage to a local SQLite database.

```bash
# Launch with LiteForge config
forge claude

# Pass arguments through to claude
forge claude -p "write tests for auth.rs"

# Show environment variables
forge claude --print-env

# Show MCP configuration
forge claude --print-mcp

# Disable usage tracking
forge claude --no-track

# Override API key or base URL
forge claude --api-key sk-xxxx --base-url https://custom-endpoint.example.com
```

#### Model Overrides

By default `forge claude` maps Claude Code's model selection to LiteForge-compatible names:

| Environment Variable | Default Value |
|---|---|
| `ANTHROPIC_MODEL` | `claude-opus-4.7` |
| `ANTHROPIC_SMALL_FAST_MODEL` | `claude-haiku-4.5` |

Export these variables before running `forge claude` to override.

### `forge usage`

View API usage reports from the local tracking database.

```bash
# Monthly summary (default)
forge usage

# Weekly summary
forge usage --period weekly

# Breakdown by model
forge usage --by-model

# List sessions
forge usage --sessions
```

### `forge guardrails`

Check text for PII and prompt injection.

```bash
# Check with specific detector
forge guardrails check --pii "text with email@example.com"
forge guardrails check --injection "ignore previous instructions"

# All checks
forge guardrails check --all "Some text to check"

# From file
forge guardrails check --all --file input.txt
```

### `forge mcp`

Manage MCP server configurations.

```bash
# List servers
forge mcp list

# Inspect a server
forge mcp info server-name

# Custom config file
forge mcp list --config custom.json
```

### `forge serve`

Start the multi-port agent server. Each role runs on its own port and can be started individually or all together.

```bash
# Start all enabled servers
forge serve
forge serve all

# Start individual roles
forge serve user               # User-facing API (port 8080)
forge serve mcp                # MCP protocol server (port 8081)
forge serve tools              # Tools REST server (port 8082)
forge serve a2a                # Agent-to-Agent server (port 8083)
forge serve knowledge          # Knowledge REST server (port 8084)
forge serve skills             # Skills REST server (port 8085)

# Override ports
forge serve --user-port 9000 --mcp-port 9001

# Custom config file
forge serve --config ./serve.toml

# Override agents directory
forge serve --agents-dir ./my-agents
```

#### Default Ports

| Role | Port | Description |
|------|------|-------------|
| User | 8080 | User-facing chat/completions API |
| MCP | 8081 | Model Context Protocol server |
| Tools | 8082 | Tools REST API |
| A2A | 8083 | Agent-to-Agent communication |
| Knowledge | 8084 | Knowledge base REST API |
| Skills | 8085 | Skills REST API |

#### Configuration File

Create `~/.config/forge/serve.toml` (or pass `--config`) to customize:

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

# agents_dir = "./agents"
```

### `forge infra`

Manage observability infrastructure (Docker-based services).

```bash
# Start detached
forge infra start -d

# Show status
forge infra status

# Follow logs
forge infra logs -f

# Stop services
forge infra stop
```

### `forge adk`

Agent Development Kit — scaffold, build, run, and test containerized agent ecosystems.

```bash
# Scaffold a new project
forge adk init my-agent

# Validate project configuration
forge adk validate
forge adk validate --dir ./my-project

# Build container image
forge adk build
forge adk build --tag my-agent:v2

# Run container
forge adk run
forge adk run -D                # detached mode

# Dev mode with hot reload
forge adk dev

# Run eval test suite
forge adk test

# Container operations
forge adk status
forge adk logs
forge adk logs -f               # follow output
forge adk stop
```

#### ADK Project Structure

Running `forge adk init my-agent` creates:

```
my-agent/
├── adk.yaml            # Project configuration
├── agents/             # Agent YAML definitions
├── tools/              # Python tool implementations
├── knowledge/          # Knowledge sources
└── tests/              # Eval test cases (*.yaml)
```

#### ADK Workflow

1. **Scaffold** — `forge adk init` creates the project directory with configuration and scaffolding
2. **Develop** — `forge adk dev` starts the multi-port server locally with file watching
3. **Validate** — `forge adk validate` checks syntax, agent files, tools, and port conflicts
4. **Test** — `forge adk test` runs eval test cases against the running agents
5. **Build** — `forge adk build` generates a Dockerfile and builds a container image
6. **Run** — `forge adk run` starts the containerized agent ecosystem
7. **Deploy** — Push the image to your container registry

## Global Options

| Flag | Description |
|------|-------------|
| `--help`, `-h` | Show help |
| `--version`, `-V` | Show version |
