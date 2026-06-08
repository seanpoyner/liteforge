# Installation

LiteForge ships as four installable artifacts. Install only what you need.

| Artifact | Install | Import / binary |
|---|---|---|
| **`forge` CLI** | install script / Homebrew / release binary | `forge` |
| **Rust crate** | `cargo add liteforge` | `use liteforge::…` |
| **Python** | `pip install liteforge` | `import liteforge` |
| **JavaScript/TS** | `npm install @seanpoyner/liteforge` | `@seanpoyner/liteforge` |

Supported toolchains: **Rust 1.70+**, **Python 3.10+**, **Node.js 18+**.

---

## CLI

### Install script (recommended)

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/seanpoyner/liteforge/main/scripts/install.sh | bash
```

```powershell
# Windows (PowerShell)
irm https://raw.githubusercontent.com/seanpoyner/liteforge/main/scripts/install.ps1 | iex
```

The installer downloads the release binary for your platform, **verifies it against the release
`SHA256SUMS` manifest (fail‑closed)**, installs to `~/.forge/bin`, and wires up your shell. It then
offers to write your API key and base URL.

> **Integrity:** if the checksum manifest can't be fetched or the artifact's hash doesn't match,
> the installer **aborts** rather than installing an unverified binary. See
> [Verifying downloads](#verifying-downloads).

### Homebrew

```bash
brew install seanpoyner/forge/forge-cli
# then
forge config init
```

### Manual binary (from a Release)

Download the asset for your platform from the
[Releases page](https://github.com/seanpoyner/liteforge/releases), verify it (below), extract, and
put `forge` on your `PATH`:

```bash
mkdir -p ~/.forge/bin
tar -xzf forge-cli-x86_64-unknown-linux-gnu.tar.gz -C ~/.forge/bin/
export PATH="$HOME/.forge/bin:$PATH"
forge --version
```

Release assets per platform:

- `forge-cli-x86_64-unknown-linux-gnu.tar.gz` (Linux x64)
- `forge-cli-aarch64-apple-darwin.tar.gz` (macOS Apple Silicon)
- `forge-cli-x86_64-apple-darwin.tar.gz` (macOS Intel)
- `forge.exe` (Windows x64)

---

## SDKs

### Rust

```bash
cargo add liteforge
```

```toml
# or in Cargo.toml
[dependencies]
liteforge = "0.2"
```

Optional OpenTelemetry export is behind a feature flag (off by default):

```toml
liteforge = { version = "0.2", features = ["otel"] }
```

### Python

```bash
pip install liteforge
```

Wheels are published for Linux (manylinux2014), macOS (arm64), and Windows on CPython 3.10–3.12.
From source instead:

```bash
pip install "git+https://github.com/seanpoyner/liteforge.git#subdirectory=crates/liteforge-py"
```

### JavaScript / TypeScript

```bash
npm install @seanpoyner/liteforge
```

Native prebuilds ship for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, and
`x86_64-pc-windows-msvc`. TypeScript types are bundled (`index.d.ts`).

---

## Verifying downloads

Every release publishes a `SHA256SUMS` manifest alongside the binaries. The install scripts verify
automatically; to check a manual download yourself:

```bash
# Download the manifest for a tag
curl -fsSLO https://github.com/seanpoyner/liteforge/releases/download/v0.2.5/SHA256SUMS

# Verify the asset you downloaded
sha256sum -c SHA256SUMS --ignore-missing
```

```powershell
# Windows
(Get-FileHash .\forge.exe -Algorithm SHA256).Hash.ToLower()
# compare against the matching line in SHA256SUMS
```

---

## Corporate CA / TLS proxies (opt‑in)

LiteForge uses **rustls** with **aws‑lc‑rs** and bundled WebPKI roots, so it does not depend on
system OpenSSL. By default the installer **does not touch your OS trust store**.

If you are behind a TLS‑inspecting proxy (Netskope, Zscaler, Forcepoint, …) and need an extra CA,
opt in explicitly:

```bash
# macOS / Linux
… install.sh | bash -s -- --with-ca-bundle
```

```powershell
# Windows
irm …/install.ps1 | iex      # then re-run with the flag, or:
.\install.ps1 -WithCaBundle
```

This installs a **LiteForge‑scoped** bundle and sets `LITEFORGE_EXTRA_CA_FILE` so the SDK/CLI add
that CA **for their own requests only** — it is *not* injected into the system trust store or other
applications. You can also set it yourself at runtime:

```bash
export LITEFORGE_EXTRA_CA_FILE=/path/to/corp-ca.pem
```

See [Configuration](Configuration) and [FAQ and Troubleshooting](FAQ-and-Troubleshooting) for more.

---

## Build from source

```bash
git clone https://github.com/seanpoyner/liteforge.git
cd liteforge

cargo build --release -p forge-cli     # CLI → target/release/forge
cargo build --release -p liteforge     # core lib
cargo test  --workspace                # tests
```

Building needs a C toolchain. On Debian/Ubuntu:

```bash
sudo apt-get install -y build-essential pkg-config libssl-dev
```

Next: **[Quickstart](Quickstart)** · **[Configuration](Configuration)**
