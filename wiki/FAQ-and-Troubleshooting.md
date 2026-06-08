# FAQ and Troubleshooting

## General

**What is LiteForge?**
A Rust SDK + CLI for building LLM apps, with Python/JS/Java bindings. It speaks the OpenAI‑compatible
protocol, so it works with OpenAI, Anthropic, LiteLLM, local Ollama, and more. See
[Architecture](Architecture).

**Which package name do I install?**

| Language | Package |
|---|---|
| Rust | `liteforge` (crates.io) |
| Python | `liteforge` (PyPI) — `import liteforge` |
| JS/TS | `@seanpoyner/liteforge` (npm) |
| CLI | install script / `seanpoyner/forge/forge-cli` (Homebrew) |

**Does it support sync and async?**
Both, in every language. `ForgeClient` is blocking; `AsyncForgeClient` is async.

## Configuration

**“No API key configured” / 401 Unauthorized.**
Set `LITEFORGE_API_KEY` (or one of the fallbacks `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`), or pass
`.api_key(...)` to the builder. Check what's resolved with `forge config show`. Precedence and
fallbacks are in [Configuration](Configuration).

**My requests go to the wrong endpoint.**
Base URL resolves `LITEFORGE_BASE_URL` → `ANTHROPIC_BASE_URL` → `OPENAI_BASE_URL` → config file →
default. If an `OPENAI_BASE_URL` is set in your shell, LiteForge will follow it. Set
`LITEFORGE_BASE_URL` explicitly to be unambiguous.

**“Model not found” / 404 from the provider.**
The model name must be one your endpoint actually serves. With a LiteLLM proxy, it's the
`model_name` from your `model_list`; with direct Ollama, it's a model you've `ollama pull`ed. See
[LiteLLM and Ollama](LiteLLM-and-Ollama).

**Requests time out on long generations.**
The timeout covers the whole request. Raise it with `LITEFORGE_TIMEOUT=120` or
`.timeout_secs(120)`. For long outputs, prefer [Streaming](Streaming).

## TLS / corporate networks

**“certificate verify failed” behind a proxy (Netskope/Zscaler/…).**
LiteForge uses its own bundled roots, not the system store. Point it at your corporate CA:

```bash
export LITEFORGE_EXTRA_CA_FILE=/path/to/corp-ca.pem
```

or re‑run the installer with `--with-ca-bundle` (`-WithCaBundle` on Windows). This adds the CA to
LiteForge's client only — it does **not** modify your OS trust store. See
[Installation → Corporate CA](Installation#corporate-ca--tls-proxies-optin).

## Installation & integrity

**The installer aborted with “refusing to install an unverified binary.”**
That's the fail‑closed checksum guard: it couldn't fetch `SHA256SUMS` or the download didn't match.
Re‑run (transient network), or download the asset and `SHA256SUMS` from the
[Releases page](https://github.com/seanpoyner/liteforge/releases) and verify manually —
see [Installation → Verifying downloads](Installation#verifying-downloads). Do not bypass the check.

**`pip install` builds from source / fails.**
Prebuilt wheels cover CPython 3.10–3.12 on Linux (manylinux2014), macOS arm64, and Windows. On other
setups pip builds from source, which needs a Rust toolchain. Use a supported interpreter or install
Rust 1.70+.

**`forge: command not found` after install.**
The binary is in `~/.forge/bin`. Add it to `PATH` (`export PATH="$HOME/.forge/bin:$PATH"`) or source
the env file the installer wrote (`source ~/.forge/env`, or `~/.forge/env.fish` for fish).

**Linux keyring errors when storing secrets.**
Install a Secret Service backend: `sudo apt-get install gnome-keyring` (Debian/Ubuntu). Secrets are
optional — config also lives in `~/.forge/config.toml`.

## Behavior

**A local model ignores my tools (no `tool_calls`).**
Tool‑calling support varies by model. Use a local model that advertises native tool calling and keep
schemas small/simple. See [Tools and Agents](Tools-and-Agents) and the local‑model note in
[LiteLLM and Ollama](LiteLLM-and-Ollama).

**Do the guardrails guarantee no PII / no injection?**
No — they're heuristic, defense‑in‑depth only. Combine with server‑side policy and human review for
anything compliance‑critical. See the warning in [Guardrails](Guardrails).

**Are prompts/outputs captured in traces?**
Not by default. Opt in with `LITEFORGE_OTEL_CAPTURE_PROMPTS=true` and only with appropriate retention
controls. See [Observability and Telemetry](Observability-and-Telemetry).

## Building from source

```bash
git clone https://github.com/seanpoyner/liteforge.git && cd liteforge
sudo apt-get install -y build-essential pkg-config libssl-dev   # Debian/Ubuntu
cargo build --release -p forge-cli
cargo test --workspace
```

Needs Rust 1.70+ and a C toolchain. More in [Installation](Installation) and
[Contributing](Contributing).

## Still stuck?

Open an issue at
[github.com/seanpoyner/liteforge/issues](https://github.com/seanpoyner/liteforge/issues) with your
OS, language/version, and the exact command + error.
