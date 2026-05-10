# Installation Guide

This guide covers installing the LiteForge components on macOS, Linux, and Windows.

## Quick Install

### macOS / Linux

```bash
git clone https://gitea.poyner.ai/sean/liteforge.git /tmp/liteforge && bash /tmp/liteforge/scripts/install.sh && rm -rf /tmp/liteforge
```

### Windows (PowerShell)

```powershell
git clone https://gitea.poyner.ai/sean/liteforge.git $env:TEMP\liteforge; & $env:TEMP\liteforge\scripts\install.ps1; Remove-Item -Recurse -Force $env:TEMP\liteforge
```

> **Note:** Building from source on Windows requires either:
> - **WSL** (Windows Subsystem for Linux) with mingw-w64 installed - the installer will automatically use WSL to cross-compile if available
> - **Visual Studio Build Tools** with C++ workload
> 
> If neither is available, the installer will attempt to download pre-built binaries from GitHub releases.

The installer will:
1. Install corporate CA certificates and auto-detect proxy CAs (e.g. Netskope, Zscaler)
2. Prompt for your LiteForge API key and configuration
3. Let you select which components to install
4. Configure your shell environment
5. Write your API key directly to `~/.forge/config.toml` and shell env files (keyring used as secondary backup when available)

## Homebrew (macOS / Linux)

For macOS and Linux users, you can also install via Homebrew:

```bash
# Add the tap
brew tap sean/forge https://gitea.poyner.ai/sean/homebrew-forge.git

# Install the CLI
brew install forge-cli
```

After installing via Homebrew, run the setup wizard:

```bash
forge config init
forge config set-secret forge-api-key
```

## GitHub CLI Install

If you have the [GitHub CLI](https://cli.github.com/) (`gh`) authenticated, you can install directly from releases:

### Windows (PowerShell)

```powershell
# Create install directory and download binary
mkdir -Force $env:USERPROFILE\.forge\bin | Out-Null
gh release download v0.1.0 --repo seanpoyner/liteforge --pattern "forge.exe" -D $env:USERPROFILE\.forge\bin

# Add to PATH (current session)
$env:PATH = "$env:USERPROFILE\.forge\bin;$env:PATH"

# Make permanent (add to PowerShell profile)
Add-Content $PROFILE '$env:PATH = "$env:USERPROFILE\.forge\bin;$env:PATH"'

# Verify
forge --version
```

### macOS / Linux

```bash
# Create install directory and download binary
mkdir -p ~/.forge/bin
gh release download v0.1.0 --repo seanpoyner/liteforge --pattern "forge-cli-x86_64-unknown-linux-gnu.tar.gz" --output - | tar -xz -C ~/.forge/bin

# Add to PATH
echo 'export PATH="$HOME/.forge/bin:$PATH"' >> ~/.bashrc  # or ~/.zshrc
source ~/.bashrc

# Verify
forge --version
```

> **Note:** This method requires `gh auth login` to authenticate with GitHub Enterprise first.

---

## Manual Installation

### Prerequisites

- **forge-cli**: No prerequisites (standalone binary)
- **liteforge-py**: Python 3.8+
- **liteforge-js**: Node.js 18+
- **liteforge (Rust)**: Rust 1.70+

### Installing forge-cli

1. Download the binary for your platform from [GitHub Releases](https://gitea.poyner.ai/sean/liteforge/releases):
   - `forge-cli-x86_64-unknown-linux-gnu.tar.gz` (Linux x64)
   - `forge-cli-aarch64-unknown-linux-gnu.tar.gz` (Linux ARM64)
   - `forge-cli-x86_64-apple-darwin.tar.gz` (macOS Intel)
   - `forge-cli-aarch64-apple-darwin.tar.gz` (macOS Apple Silicon)
   - `forge.exe` (Windows x64)

2. Extract and install:

   **macOS / Linux:**
   ```bash
   mkdir -p ~/.forge/bin
   tar -xzf forge-cli-*.tar.gz -C ~/.forge/bin/
   chmod +x ~/.forge/bin/forge
   export PATH="$HOME/.forge/bin:$PATH"
   ```

   **Windows:**
   ```powershell
   mkdir -Force $env:USERPROFILE\.forge\bin | Out-Null
   Move-Item forge.exe $env:USERPROFILE\.forge\bin\
   $env:PATH = "$env:USERPROFILE\.forge\bin;$env:PATH"
   ```

3. Verify installation:
   ```bash
   forge --version
   ```

### Installing liteforge-py (Python)

**From GitHub Release:**
```bash
pip install https://gitea.poyner.ai/sean/liteforge/releases/latest/download/liteforge-py3-none-any.whl
```

**From Git:**
```bash
pip install "git+https://gitea.poyner.ai/sean/liteforge.git#subdirectory=crates/liteforge-py"
```

### Installing liteforge-js (Node.js)

**From GitHub Release:**
```bash
npm install https://gitea.poyner.ai/sean/liteforge/releases/latest/download/liteforge.tgz
```

**From Git:**
```bash
npm install "git+https://gitea.poyner.ai/sean/liteforge.git#subdirectory=crates/liteforge-js"
```

### Installing liteforge (Rust)

Add to your `Cargo.toml`:

```toml
[dependencies]
liteforge = { git = "https://gitea.poyner.ai/sean/liteforge.git" }
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LITEFORGE_API_KEY` | API key for authentication | Required |
| `LITEFORGE_BASE_URL` | LiteLLM endpoint URL | LiteForge production endpoint |
| `LITEFORGE_DEFAULT_MODEL` | Default model for completions | `anthropic.claude-haiku-4-5-20251001-v1:0` |
| `LITEFORGE_TIMEOUT` | Request timeout in seconds | `60` |
| `LITEFORGE_KNOWLEDGE_URL` | Knowledge service endpoint | Optional |
| `LITEFORGE_TEMPORAL_URL` | Temporal endpoint | Optional |

### Credential Storage

The installer writes your API key directly to `~/.forge/config.toml` and shell env files (`~/.forge/env`, `~/.forge/env.fish`) so the key is available immediately without external dependencies. If the `forge` CLI is installed, the key is also stored in the platform's native credential manager as a secondary mechanism:

| Platform | Secondary Storage |
|----------|---------|
| macOS | Keychain |
| Linux | Secret Service (GNOME Keyring, KWallet) |
| Windows | Credential Manager |

To manage stored secrets:

```bash
# Store a secret
forge config set-secret forge-api-key

# Retrieve a secret
forge config get-secret forge-api-key

# List all secrets
forge config list-secrets

# Delete a secret
forge config delete-secret forge-api-key
```

### Config File

The CLI uses `~/.forge/config.toml` for configuration:

```toml
api_key = "your-api-key"

[endpoints]
base_url = "https://api.example.com/v1"
knowledge_url = ""
temporal_url = ""

[defaults]
model = "anthropic.claude-haiku-4-5-20251001-v1:0"
timeout = 60

[paths]
# agents_dir = "~/.forge/agents"
# skills_dir = "~/.forge/skills"
```

## CA Certificates

The installer includes corporate CA certificates for secure connections to internal services. It also **auto-detects proxy CAs** (e.g. Netskope, Zscaler, Forcepoint) by inspecting TLS connections to known registries and adds them to the trust store automatically. Certificates are installed to:

- **macOS**: System Keychain
- **Linux**: `/usr/local/share/ca-certificates/`
- **Windows**: Current User certificate store (required for cargo/Rust builds)

Environment variables are also set for tools that use their own certificate stores:
- `SSL_CERT_FILE` (set conditionally — only if not already defined by the user)
- `REQUESTS_CA_BUNDLE`
- `NODE_EXTRA_CA_CERTS`

!!! note
    On Windows, `CARGO_HTTP_CHECK_REVOKE` is set to `false` because proxy certificates often fail revocation checks. The installer does **not** set `CARGO_HTTP_CAINFO` so that cargo uses the Windows certificate store directly (which already contains the proxy CAs).

## Troubleshooting

### "Certificate verify failed" errors

Ensure the CA certificates are installed:
```bash
# Check if cert file exists
ls ~/.forge/certs/ca-bundle.crt

# Verify environment variable
echo $SSL_CERT_FILE
```

### Keyring errors on Linux

Install the Secret Service backend:
```bash
# Ubuntu/Debian
sudo apt-get install gnome-keyring

# Fedora
sudo dnf install gnome-keyring
```

### PATH not updated

Add to your shell profile manually:

**Bash (~/.bashrc):**
```bash
source "$HOME/.forge/env"
```

**Zsh (~/.zshrc):**
```bash
source "$HOME/.forge/env"
```

**Fish (~/.config/fish/config.fish):**
```fish
source "$HOME/.forge/env.fish"
```

**PowerShell ($PROFILE):**
```powershell
$env:PATH = "$env:USERPROFILE\.forge\bin;$env:PATH"
```

## Build from Source

All installers support `--build-from-source` for environments where pre-built binaries are not available:

```bash
# macOS / Linux
git clone https://gitea.poyner.ai/sean/liteforge.git /tmp/liteforge
bash /tmp/liteforge/scripts/install.sh --build-from-source

# Windows (requires WSL)
git clone https://gitea.poyner.ai/sean/liteforge.git $env:TEMP\liteforge
& $env:TEMP\liteforge\scripts\install.ps1 -BuildFromSource
```

### Prerequisites

- **Rust 1.70+** (all platforms)
- **A C toolchain** — required by Rust to link native code (see platform-specific sections below)
- **Python 3.8+** (for `liteforge-py`)
- **Node.js 18+** (for `liteforge-js`)

### Linux Build Requirements

Building from source on Linux requires a C compiler and linker (`cc`/`gcc`). On Debian/Ubuntu:

```bash
sudo apt-get update && sudo apt-get install -y build-essential pkg-config libssl-dev
```

The installer will detect a missing C toolchain and offer to install these packages automatically (requires sudo).

### Windows Build Requirements

On Windows, building from source requires one of:

1. **Visual Studio Build Tools** with the "Desktop development with C++" workload — the installer will auto-detect existing installations and offer to download Build Tools if missing
2. **WSL (Windows Subsystem for Linux)** with `mingw-w64` — the installer will automatically cross-compile via WSL if available

If neither is available, the installer will attempt to download pre-built binaries from GitHub releases.

To set up WSL manually:
```powershell
# Enable WSL (requires restart)
wsl --install

# After restart, install build tools in WSL
wsl sudo apt-get update && sudo apt-get install -y mingw-w64
wsl rustup target add x86_64-pc-windows-gnu
```

Alternatively, you can install Visual Studio Build Tools with the "Desktop development with C++" workload for native MSVC builds.

## Upgrading

To upgrade to the latest version, re-run the installer:

```bash
# macOS / Linux
git clone https://gitea.poyner.ai/sean/liteforge.git /tmp/liteforge && bash /tmp/liteforge/scripts/install.sh && rm -rf /tmp/liteforge

# Windows
git clone https://gitea.poyner.ai/sean/liteforge.git $env:TEMP\liteforge; & $env:TEMP\liteforge\scripts\install.ps1; Remove-Item -Recurse -Force $env:TEMP\liteforge
```

Or with Homebrew:
```bash
brew upgrade forge-cli
```

## Uninstalling

To remove LiteForge:

1. Remove the installation directory:
   ```bash
   rm -rf ~/.forge
   ```

2. Remove the line from your shell profile that sources `~/.forge/env`

3. (Optional) Remove stored credentials:
   - **macOS**: Use Keychain Access to delete "forge" entries
   - **Linux**: Use Seahorse or `secret-tool` to delete entries
   - **Windows**: Use Credential Manager to delete "forge" entries
