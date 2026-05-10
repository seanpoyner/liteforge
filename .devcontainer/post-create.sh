#!/usr/bin/env bash
# Runs once after the container is first created.
set -euo pipefail

# Named volumes are root-owned by default — chown to the vscode user.
sudo chown -R vscode:vscode \
    /usr/local/cargo/registry \
    /usr/local/cargo/git \
    "${PWD}/target" 2>/dev/null || true

# Rust components: clippy + rustfmt for IDE / CI integration.
rustup component add clippy rustfmt

# Python: maturin for building / publishing the PyO3 wheel.
pip install --user --upgrade pip maturin

# Node: napi-rs CLI for the JS binding build.
sudo npm install -g @napi-rs/cli

echo
echo "Toolchain versions:"
rustc --version
cargo --version
python3 --version
node --version
java -version 2>&1
gradle --version 2>/dev/null | head -3 || true
