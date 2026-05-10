#!/bin/bash
set -e

# Add Netskope proxy CA to cert bundle
cp /etc/ssl/certs/ca-certificates.crt /tmp/ca-bundle.crt
openssl s_client -showcerts -connect index.crates.io:443 -servername index.crates.io </dev/null 2>/dev/null \
  | sed -ne '/-BEGIN CERTIFICATE-/,/-END CERTIFICATE-/p' >> /tmp/ca-bundle.crt

export SSL_CERT_FILE=/tmp/ca-bundle.crt
export CARGO_HTTP_CAINFO=/tmp/ca-bundle.crt
export CARGO_HTTP_TIMEOUT=120

# Ensure latest stable Rust
rustup default stable

# Build the JS bindings
cargo build -p liteforge-js --release

echo "=== BUILD OK ==="

# Copy the shared object as a .node file
cp target/release/libliteforge_js.so crates/liteforge-js/liteforge.linux-x64-gnu.node

ls -la crates/liteforge-js/*.node

# Quick smoke test: load the module
node -e 'const m = require("./crates/liteforge-js/liteforge.linux-x64-gnu.node"); console.log("Loaded! Exports:", Object.keys(m).slice(0, 15).join(", "))'
