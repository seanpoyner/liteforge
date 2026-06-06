# Installation

For a complete guide covering all platforms and install methods, see the [Installation Guide](installation.md).

## Quick Install

=== "macOS / Linux"

    ```bash
    git clone https://github.com/seanpoyner/liteforge.git /tmp/liteforge && bash /tmp/liteforge/scripts/install.sh && rm -rf /tmp/liteforge
    ```

=== "Windows (PowerShell)"

    ```powershell
    git clone https://github.com/seanpoyner/liteforge.git $env:TEMP\liteforge; & $env:TEMP\liteforge\scripts\install.ps1; Remove-Item -Recurse -Force $env:TEMP\liteforge
    ```

=== "Homebrew"

    ```bash
    brew tap seanpoyner/forge https://github.com/seanpoyner/homebrew-forge.git
    brew install forge-cli
    ```

## Rust

Add `liteforge` to your project's `Cargo.toml`:

```toml
[dependencies]
liteforge = { git = "https://github.com/seanpoyner/liteforge.git" }
```

### Build from Source

```bash
git clone https://github.com/seanpoyner/liteforge.git
cd liteforge
cargo build --all
```

## Python

The Python bindings are built with [maturin](https://www.maturin.rs/) and PyO3.

### Development Install

```bash
cd crates/liteforge-py
pip install maturin
maturin develop
```

### Build a Wheel

```bash
cd crates/liteforge-py
maturin build --release
pip install target/wheels/liteforge-*.whl
```

## JavaScript / TypeScript

The JavaScript/TypeScript bindings are built with [napi-rs](https://napi.rs), producing a native `.node` addon with auto-generated TypeScript definitions.

### Development Install

```bash
cd crates/liteforge-js
npm install
npm run build
```

This produces:

- A platform-specific `.node` native addon
- `index.d.ts` — auto-generated TypeScript type definitions
- `index.js` — JavaScript entry point

### Using in Your Project

Link the built package into your project:

```bash
npm link ../path-to/liteforge/crates/liteforge-js
```

Or reference it directly in your `package.json`:

```json
{
  "dependencies": {
    "@forge/sdk": "file:../liteforge/crates/liteforge-js"
  }
}
```

## CLI

The `forge` CLI is installed automatically by the quick install scripts, or can be built manually:

```bash
cargo install --path crates/forge-cli
```

After installation, verify with:

```bash
forge --version
forge --help
```

See the [CLI Reference](cli.md) for the full list of commands including `forge adk`, `forge serve`, and `forge models`.

## Environment Setup

Set your LiteForge API key before using the SDK:

=== "Environment Variable"

    ```bash
    export LITEFORGE_API_KEY="your-api-key-here"
    ```

=== "`.env` File"

    Create a `.env` file in your project root:

    ```env
    LITEFORGE_API_KEY=your-api-key-here
    ```

The SDK automatically loads `.env` files via `dotenvy`.

## Requirements

| Component | Minimum Version |
|-----------|----------------|
| Rust | 1.70+ (edition 2021) |
| Python | 3.8+ (for bindings) |
| Node.js | 18+ (for JS/TS bindings) |
| OpenSSL | Not required (uses `rustls`) |
