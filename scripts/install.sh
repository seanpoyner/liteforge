#!/usr/bin/env bash
#
# LiteForge Installer for macOS and Linux
#
# Usage (recommended - downloads pre-built binary, falls back to source build):
#   git clone https://github.com/seanpoyner/liteforge.git /tmp/liteforge
#   bash /tmp/liteforge/scripts/install.sh
#   rm -rf /tmp/liteforge
#
# Or as one-liner:
#   git clone https://github.com/seanpoyner/liteforge.git /tmp/liteforge && bash /tmp/liteforge/scripts/install.sh && rm -rf /tmp/liteforge
#
# To force building from source (requires a C toolchain):
#   LITEFORGE_BUILD_FROM_SOURCE=1 bash /tmp/liteforge/scripts/install.sh
#   # or: bash /tmp/liteforge/scripts/install.sh --build-from-source
#

set -euo pipefail

# When run via a pipe (curl -fsSL .../install.sh | bash), stdin is the script
# itself, so interactive `read` prompts would fail. Reconnect stdin to the
# terminal if one is available; otherwise prompts fall back / require
# --non-interactive with env vars.
if [ ! -t 0 ] && [ -r /dev/tty ]; then
    exec < /dev/tty
fi

# ============================================================================
# Configuration
# ============================================================================

LITEFORGE_VERSION="${LITEFORGE_VERSION:-latest}"
LITEFORGE_HOME="${LITEFORGE_HOME:-$HOME/.forge}"
GITHUB_BASE_URL="${GITHUB_BASE_URL:-https://api.github.com/repos/seanpoyner/liteforge}"
GITHUB_RELEASE_URL="${GITHUB_RELEASE_URL:-https://github.com/seanpoyner/liteforge/releases}"

# Detect if running from a git clone (for building from source)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." 2>/dev/null && pwd)" || REPO_ROOT=""
IN_REPO=false
if [[ -n "$REPO_ROOT" && -f "$REPO_ROOT/Cargo.toml" ]]; then
    IN_REPO=true
fi

# Default configuration values
DEFAULT_BASE_URL="https://api.example.com/v1"
DEFAULT_MODEL="anthropic.claude-haiku-4-5-20251001-v1:0"

# Colors (disabled if not a tty)
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    CYAN='\033[0;36m'
    BOLD='\033[1m'
    DIM='\033[2m'
    NC='\033[0m' # No Color
else
    RED='' GREEN='' YELLOW='' BLUE='' CYAN='' BOLD='' DIM='' NC=''
fi

# ============================================================================
# Helper Functions
# ============================================================================

info() {
    echo -e "${BLUE}==>${NC} ${BOLD}$1${NC}"
}

success() {
    echo -e "${GREEN}✓${NC} $1"
}

warn() {
    echo -e "${YELLOW}!${NC} $1"
}

error() {
    echo -e "${RED}✗${NC} $1" >&2
}

die() {
    error "$1"
    exit 1
}

prompt() {
    local var_name="$1"
    local prompt_text="$2"
    local default="${3:-}"
    local is_secret="${4:-false}"

    if [[ -n "$default" ]]; then
        prompt_text="$prompt_text [${DIM}$default${NC}]"
    fi

    echo -en "${CYAN}?${NC} $prompt_text: "

    if [[ "$is_secret" == "true" ]]; then
        read -rs value
        echo
    else
        read -r value
    fi

    if [[ -z "$value" && -n "$default" ]]; then
        value="$default"
    fi

    eval "$var_name=\"\$value\""
}

confirm() {
    local prompt_text="$1"
    local default="${2:-n}"

    if [[ "$default" == "y" ]]; then
        prompt_text="$prompt_text [Y/n]"
    else
        prompt_text="$prompt_text [y/N]"
    fi

    echo -en "${CYAN}?${NC} $prompt_text: "
    read -r response

    if [[ -z "$response" ]]; then
        response="$default"
    fi

    [[ "$response" =~ ^[Yy] ]]
}

# Install a pip package, handling PEP 668 externally-managed environments
# Uses python3 -m pip to ensure pip matches the detected python interpreter.
install_pip_package() {
    local package="$1"
    shift
    # Note: ${arr[@]+"${arr[@]}"} is the nounset-safe empty-array expansion.
    # Required for macOS's stock Bash 3.2, which treats "${arr[@]}" as unbound when empty.
    local extra_args=()
    if [[ $# -gt 0 ]]; then
        extra_args=("$@")
    fi

    # If in a virtual environment, install directly
    if [[ -n "${VIRTUAL_ENV:-}" ]] || python3 -c "import sys; exit(0 if sys.prefix != sys.base_prefix else 1)" 2>/dev/null; then
        python3 -m pip install ${extra_args[@]+"${extra_args[@]}"} "$package"
        return $?
    fi

    # Try --user first
    if python3 -m pip install --user ${extra_args[@]+"${extra_args[@]}"} "$package" 2>/dev/null; then
        return 0
    fi

    # If that failed, try with --break-system-packages (PEP 668)
    warn "System Python is externally managed (PEP 668)"
    if confirm "Install with --break-system-packages?" "y"; then
        python3 -m pip install --user --break-system-packages ${extra_args[@]+"${extra_args[@]}"} "$package"
        return $?
    fi

    # Suggest alternatives
    echo
    echo "  Alternatives:"
    echo "    1. Create a virtual environment: ${CYAN}python3 -m venv ~/.forge/venv${NC}"
    echo "    2. Use pipx: ${CYAN}pipx install $package${NC}"
    echo
    return 1
}

# ============================================================================
# Platform Detection
# ============================================================================

detect_platform() {
    local os arch

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            OS="linux"
            ;;
        Darwin)
            OS="darwin"
            ;;
        *)
            die "Unsupported operating system: $os"
            ;;
    esac

    case "$arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        arm64|aarch64)
            ARCH="aarch64"
            ;;
        *)
            die "Unsupported architecture: $arch"
            ;;
    esac

    # Construct target triple
    if [[ "$OS" == "linux" ]]; then
        TARGET="${ARCH}-unknown-linux-gnu"
    else
        TARGET="${ARCH}-apple-darwin"
    fi

    success "Detected platform: $OS ($ARCH)"
}

# ============================================================================
# Prerequisite Checks
# ============================================================================

check_prerequisites() {
    info "Checking prerequisites..."

    local missing=()

    # Required tools
    command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 || missing+=("curl or wget")
    command -v tar >/dev/null 2>&1 || missing+=("tar")

    # Optional tools (for SDK installation)
    if [[ -z "${SKIP_PYTHON:-}" ]]; then
        if ! command -v python3 >/dev/null 2>&1; then
            warn "python3 not found - Python SDK installation will be skipped"
        fi
    fi

    if [[ -z "${SKIP_NODE:-}" ]]; then
        if ! command -v node >/dev/null 2>&1 || ! command -v npm >/dev/null 2>&1; then
            warn "node/npm not found - Node.js SDK installation will be skipped"
        fi
    fi

    if [[ -z "${SKIP_RUST:-}" ]]; then
        if ! command -v cargo >/dev/null 2>&1; then
            warn "cargo not found - Rust SDK installation will provide git instructions only"
        fi
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        die "Missing required tools: ${missing[*]+${missing[*]}}"
    fi

    success "All required tools available"
}

# ============================================================================
# CA Certificate Installation
# ============================================================================

install_ca_certs() {
    info "Installing a LiteForge-scoped CA bundle (no changes to the system trust store)..."

    local cert_source=""
    local cert_dest="$LITEFORGE_HOME/certs/ca-bundle.crt"

    mkdir -p "$LITEFORGE_HOME/certs"

    # Check for local cert bundle when running from git clone
    if $IN_REPO && [[ -f "$REPO_ROOT/certs/combined-full-ca-bundle.crt" ]]; then
        cert_source="$REPO_ROOT/certs/combined-full-ca-bundle.crt"
    elif [[ -f "certs/combined-full-ca-bundle.crt" ]]; then
        cert_source="certs/combined-full-ca-bundle.crt"
    fi

    # Download cert bundle if not found locally
    if [[ -z "$cert_source" ]]; then
        local cert_url="${GITHUB_RELEASE_URL}/download/v${LITEFORGE_VERSION}/ca-bundle.crt"
        if [[ "$LITEFORGE_VERSION" == "latest" ]]; then
            cert_url="${GITHUB_RELEASE_URL}/latest/download/ca-bundle.crt"
        fi

        if command -v curl >/dev/null 2>&1; then
            curl -sSfL "$cert_url" -o "$cert_dest" 2>/dev/null || {
                warn "Could not download CA bundle - skipping cert installation"
                return 0
            }
        else
            wget -q "$cert_url" -O "$cert_dest" 2>/dev/null || {
                warn "Could not download CA bundle - skipping cert installation"
                return 0
            }
        fi
    else
        cp "$cert_source" "$cert_dest"
    fi

    success "CA bundle written to $cert_dest"

    # Scoped to LiteForge only. The bundle is exposed to the forge CLI and the
    # Python/Node SDKs through environment variables in ~/.forge/env (see
    # write_env_file): LITEFORGE_EXTRA_CA_FILE for the Rust client, and
    # SSL_CERT_FILE / REQUESTS_CA_BUNDLE / NODE_EXTRA_CA_CERTS for the bindings.
    # We deliberately do NOT add these certs to the OS trust store, so this
    # install never widens the trust base of the whole machine.
    info "These certificates are trusted by LiteForge only, not the system trust store."
}

# ============================================================================
# Configuration Collection
# ============================================================================

collect_configuration() {
    info "Configuring LiteForge..."
    echo

    # LiteForge API Key (required)
    prompt LITEFORGE_API_KEY "LiteForge API Key" "" "true"
    if [[ -z "$LITEFORGE_API_KEY" ]]; then
        die "LiteForge API Key is required"
    fi

    # Base URL (optional)
    prompt LITEFORGE_BASE_URL "LiteForge Base URL" "$DEFAULT_BASE_URL"

    # Default model (optional)
    prompt LITEFORGE_DEFAULT_MODEL "Default Model" "$DEFAULT_MODEL"

    # Optional services
    echo
    info "Optional service endpoints (press Enter to skip):"
    prompt LITEFORGE_KNOWLEDGE_URL "Knowledge Service URL" ""
    prompt LITEFORGE_TEMPORAL_URL "Temporal Endpoint" ""

    echo
}

# ============================================================================
# Credential Validation
# ============================================================================

validate_credentials() {
    info "Validating credentials..."

    # Simple health check against the base URL
    local health_url="${LITEFORGE_BASE_URL%/}/health"
    local response

    if command -v curl >/dev/null 2>&1; then
        response=$(curl -sSf -H "Authorization: Bearer $LITEFORGE_API_KEY" "$health_url" 2>&1) || {
            warn "Could not validate LiteForge API credentials"
            if confirm "Continue anyway?" "n"; then
                return 0
            else
                die "Credential validation failed"
            fi
        }
    fi

    success "Credentials validated successfully"
}

# ============================================================================
# Component Selection
# ============================================================================

select_components() {
    info "Select components to install:"
    echo

    INSTALL_CLI=false
    INSTALL_PY=false
    INSTALL_JS=false
    INSTALL_RS=false

    if confirm "  forge-cli (CLI tool)" "y"; then
        INSTALL_CLI=true
    fi

    if command -v python3 >/dev/null 2>&1; then
        if confirm "  liteforge-py (Python SDK)" "n"; then
            INSTALL_PY=true
        fi
    fi

    if command -v npm >/dev/null 2>&1; then
        if confirm "  liteforge-js (Node.js SDK)" "n"; then
            INSTALL_JS=true
        fi
    fi

    if command -v cargo >/dev/null 2>&1; then
        if confirm "  liteforge (Rust SDK)" "n"; then
            INSTALL_RS=true
        fi
    fi

    if ! $INSTALL_CLI && ! $INSTALL_PY && ! $INSTALL_JS && ! $INSTALL_RS; then
        warn "No components selected"
        if ! confirm "Continue with configuration only?" "n"; then
            exit 0
        fi
    fi

    echo
}

# ============================================================================
# Toolchain Pre-flight
# ============================================================================

need_source_build_toolchain() {
    if command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1; then
        return 0
    fi

    warn "No C linker (cc/gcc) found -- required to build forge-cli from source"

    if [[ "$OS" == "linux" ]] && command -v apt-get >/dev/null 2>&1; then
        if confirm "Install build-essential via sudo apt-get?" "y"; then
            sudo apt-get update && sudo apt-get install -y build-essential pkg-config libssl-dev || \
                die "Failed to install build-essential"
            success "C toolchain installed"
            return 0
        fi
    elif [[ "$OS" == "darwin" ]]; then
        echo
        echo "  Install Xcode Command Line Tools:"
        echo "    ${CYAN}xcode-select --install${NC}"
        echo
    fi

    die "A C toolchain is required to build from source. On Debian/Ubuntu: sudo apt-get install build-essential"
}

need_pip() {
    if python3 -m pip --version >/dev/null 2>&1; then
        return 0
    fi

    warn "pip not found -- required to install the Python SDK"

    if [[ "$OS" == "linux" ]] && command -v apt-get >/dev/null 2>&1; then
        if confirm "Install python3-pip via sudo apt-get?" "y"; then
            sudo apt-get update && sudo apt-get install -y python3-pip python3-venv || \
                die "Failed to install python3-pip"
            success "pip installed"
            return 0
        fi
    elif [[ "$OS" == "darwin" ]]; then
        echo
        echo "  Install pip via:"
        echo "    ${CYAN}python3 -m ensurepip --upgrade${NC}"
        echo
    fi

    die "pip is required for Python SDK installation. On Debian/Ubuntu: sudo apt-get install python3-pip"
}

# ============================================================================
# Component Installation
# ============================================================================

# Verify a downloaded artifact against the release SHA256SUMS manifest.
# Fails closed: if the manifest cannot be fetched, has no entry for the asset,
# or the hash does not match, the install aborts rather than running an
# unverified binary.
verify_sha256() {
    local file="$1" asset="$2"
    local sums_url
    if [[ "$LITEFORGE_VERSION" == "latest" ]]; then
        sums_url="${GITHUB_RELEASE_URL}/latest/download/SHA256SUMS"
    else
        sums_url="${GITHUB_RELEASE_URL}/download/v${LITEFORGE_VERSION}/SHA256SUMS"
    fi

    local sums
    if command -v curl >/dev/null 2>&1; then
        sums=$(curl -sSfL "$sums_url" 2>/dev/null) || die "Could not fetch SHA256SUMS from $sums_url; refusing to install an unverified binary"
    else
        sums=$(wget -qO- "$sums_url" 2>/dev/null) || die "Could not fetch SHA256SUMS from $sums_url; refusing to install an unverified binary"
    fi

    local expected
    expected=$(printf '%s\n' "$sums" | awk -v a="$asset" '$2 == a || $2 == "*" a {print $1; exit}')
    [[ -n "$expected" ]] || die "No checksum entry for $asset in SHA256SUMS; refusing to install an unverified binary"

    local actual
    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$file" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        die "Need 'sha256sum' or 'shasum' to verify download integrity"
    fi

    [[ "$actual" == "$expected" ]] || die "Checksum mismatch for $asset (expected $expected, got $actual); aborting"
    success "Verified $asset (sha256)"
}

try_install_prebuilt_cli() {
    local bin_dir="$1"

    local archive_name="forge-cli-${TARGET}.tar.gz"
    local download_url

    if [[ "$LITEFORGE_VERSION" == "latest" ]]; then
        download_url="${GITHUB_RELEASE_URL}/latest/download/${archive_name}"
    else
        download_url="${GITHUB_RELEASE_URL}/download/v${LITEFORGE_VERSION}/${archive_name}"
    fi

    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT

    info "Downloading pre-built binary from $download_url..."

    if command -v curl >/dev/null 2>&1; then
        curl -sSfL "$download_url" -o "$tmp_dir/forge-cli.tar.gz" 2>/dev/null || return 1
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$download_url" -O "$tmp_dir/forge-cli.tar.gz" 2>/dev/null || return 1
    else
        return 1
    fi

    # Verify integrity before extracting/executing. This aborts on mismatch.
    verify_sha256 "$tmp_dir/forge-cli.tar.gz" "$archive_name"

    tar -xzf "$tmp_dir/forge-cli.tar.gz" -C "$tmp_dir" || return 1

    local binary
    binary=$(find "$tmp_dir" -name "forge" -type f | head -1)
    if [[ -z "$binary" ]]; then
        return 1
    fi

    chmod +x "$binary"
    mv "$binary" "$bin_dir/forge"

    success "forge-cli installed to $bin_dir/forge (pre-built)"
    return 0
}

install_cli_from_source() {
    local bin_dir="$1"

    if ! command -v cargo >/dev/null 2>&1; then
        die "cargo not found -- install Rust (https://rustup.rs) to build from source"
    fi

    need_source_build_toolchain

    if $IN_REPO; then
        info "Building forge-cli from source..."
        (cd "$REPO_ROOT" && cargo build --release --package forge-cli) || die "Failed to build forge-cli"

        local binary="$REPO_ROOT/target/release/forge"
        if [[ ! -f "$binary" ]]; then
            die "Build succeeded but binary not found at $binary"
        fi

        cp "$binary" "$bin_dir/forge"
        chmod +x "$bin_dir/forge"
        success "forge-cli built and installed to $bin_dir/forge"
    else
        die "Source build requires a git clone of the repository"
    fi
}

install_cli() {
    if ! $INSTALL_CLI; then
        return 0
    fi

    info "Installing forge-cli..."

    local bin_dir="$LITEFORGE_HOME/bin"
    mkdir -p "$bin_dir"

    if [[ "${LITEFORGE_BUILD_FROM_SOURCE:-0}" == "1" ]]; then
        install_cli_from_source "$bin_dir"
        return 0
    fi

    if try_install_prebuilt_cli "$bin_dir"; then
        return 0
    fi

    warn "Pre-built binary unavailable, falling back to source build"

    if $IN_REPO && command -v cargo >/dev/null 2>&1; then
        install_cli_from_source "$bin_dir"
    else
        error "Could not download pre-built binary and source build is not available."
        echo
        echo "  Options:"
        echo "    1. Re-run with source build:  ${CYAN}LITEFORGE_BUILD_FROM_SOURCE=1 bash $0${NC}"
        echo "    2. Clone the repo and retry:  ${CYAN}git clone https://github.com/seanpoyner/liteforge.git /tmp/liteforge${NC}"
        echo "       ${CYAN}LITEFORGE_BUILD_FROM_SOURCE=1 bash /tmp/liteforge/scripts/install.sh${NC}"
        echo
        die "Cannot install forge-cli"
    fi
}

install_python_sdk() {
    if ! $INSTALL_PY; then
        return 0
    fi

    info "Installing liteforge-py..."

    need_pip

    # pyo3 0.21 hard-caps at Python 3.12. On newer interpreters the build aborts
    # unless we opt into the stable-ABI forward-compat shim. Safe to set blindly:
    # it's a no-op on supported versions.
    export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1

    # If running from a git clone, build from source
    if $IN_REPO && [[ -d "$REPO_ROOT/crates/liteforge-py" ]]; then
        if command -v maturin >/dev/null 2>&1; then
            info "Building liteforge-py from source with maturin..."
            (cd "$REPO_ROOT/crates/liteforge-py" && maturin build --release) || {
                warn "maturin build failed, trying pip install from local path..."
            }
            # maturin puts wheels in workspace root target/wheels/ or crate target/wheels/
            local wheel=""
            for wheel_dir in "$REPO_ROOT/target/wheels" "$REPO_ROOT/crates/liteforge-py/target/wheels"; do
                if [[ -d "$wheel_dir" ]]; then
                    wheel=$(find "$wheel_dir" -name "*.whl" 2>/dev/null | head -1) || true
                    [[ -n "$wheel" ]] && break
                fi
            done
            if [[ -n "$wheel" ]]; then
                install_pip_package "$wheel" --force-reinstall || die "Failed to install built wheel"
                success "liteforge-py built and installed"
                return 0
            fi
        fi

        # Fallback: try pip install from local directory
        info "Installing liteforge-py from local source..."
        install_pip_package "$REPO_ROOT/crates/liteforge-py" || {
            error "Failed to install Python SDK from source"
            return 1
        }
        success "liteforge-py installed from source"
        return 0
    fi

    # Try downloading from releases (requires auth on GHE)
    local wheel_name="liteforge-${LITEFORGE_VERSION#v}-py3-none-any.whl"
    local download_url

    if [[ "$LITEFORGE_VERSION" == "latest" ]]; then
        download_url="${GITHUB_RELEASE_URL}/latest/download/liteforge-py3-none-any.whl"
    else
        download_url="${GITHUB_RELEASE_URL}/download/v${LITEFORGE_VERSION}/${wheel_name}"
    fi

    install_pip_package "$download_url" 2>/dev/null || {
        warn "Could not install from wheel (GHE requires auth), trying git install..."
        install_pip_package "git+https://github.com/seanpoyner/liteforge.git#subdirectory=crates/liteforge-py" || {
            error "Failed to install Python SDK"
            return 1
        }
    }

    success "liteforge-py installed"
}

install_node_sdk() {
    if ! $INSTALL_JS; then
        return 0
    fi

    info "Installing liteforge-js..."

    # Set up user-local npm prefix to avoid permission issues
    local npm_prefix="$LITEFORGE_HOME/node"
    mkdir -p "$npm_prefix"

    # Helper to install npm package globally to user prefix
    npm_install_global() {
        npm install --prefix "$npm_prefix" "$@"
    }

    # If running from a git clone, build from source
    if $IN_REPO && [[ -d "$REPO_ROOT/crates/liteforge-js" ]]; then
        info "Building liteforge-js from source..."
        (cd "$REPO_ROOT/crates/liteforge-js" && npm install && npm run build) || {
            error "Failed to build liteforge-js"
            return 1
        }

        info "Installing built package to $npm_prefix..."
        (cd "$REPO_ROOT/crates/liteforge-js" && npm pack && npm_install_global ./*.tgz) || {
            error "Failed to install liteforge-js package"
            return 1
        }
        success "liteforge-js built and installed"
        return 0
    fi

    # Try downloading from releases (requires auth on GHE)
    local pkg_name="liteforge-${LITEFORGE_VERSION#v}.tgz"
    local download_url

    if [[ "$LITEFORGE_VERSION" == "latest" ]]; then
        download_url="${GITHUB_RELEASE_URL}/latest/download/liteforge.tgz"
    else
        download_url="${GITHUB_RELEASE_URL}/download/v${LITEFORGE_VERSION}/${pkg_name}"
    fi

    npm_install_global "$download_url" 2>/dev/null || {
        warn "Could not install from tgz (GHE requires auth), trying git..."
        npm_install_global "git+https://github.com/seanpoyner/liteforge.git#subdirectory=crates/liteforge-js" || {
            error "Failed to install Node.js SDK"
            return 1
        }
    }

    success "liteforge-js installed"
}

install_rust_sdk() {
    if ! $INSTALL_RS; then
        return 0
    fi

    info "Rust SDK installation..."
    echo
    echo "  Add to your Cargo.toml:"
    echo
    echo "    ${CYAN}[dependencies]${NC}"
    echo "    ${CYAN}liteforge = { git = \"https://github.com/seanpoyner/liteforge.git\" }${NC}"
    echo

    success "Rust SDK instructions provided"
}

# ============================================================================
# Configuration File Setup
# ============================================================================

write_configuration() {
    info "Writing configuration files..."

    mkdir -p "$LITEFORGE_HOME"

    # Write config.toml
    cat > "$LITEFORGE_HOME/config.toml" << EOF
# LiteForge Configuration
# Generated by install.sh on $(date -Iseconds)

api_key = "$LITEFORGE_API_KEY"

[endpoints]
base_url = "$LITEFORGE_BASE_URL"
knowledge_url = "${LITEFORGE_KNOWLEDGE_URL:-}"
temporal_url = "${LITEFORGE_TEMPORAL_URL:-}"

[defaults]
model = "$LITEFORGE_DEFAULT_MODEL"
timeout = 60

[paths]
# Uncomment to override default paths:
# agents_dir = "$LITEFORGE_HOME/agents"
# skills_dir = "$LITEFORGE_HOME/skills"
# mcp_dir = "$LITEFORGE_HOME/mcp"
# tools_dir = "$LITEFORGE_HOME/tools"
EOF

    success "Created $LITEFORGE_HOME/config.toml"

    # Write env file (non-sensitive values only)
    cat > "$LITEFORGE_HOME/env" << EOF
# LiteForge Environment Variables
# Generated by install.sh on $(date -Iseconds)
# Source this file from your shell profile

# Non-sensitive configuration
export LITEFORGE_API_KEY="$LITEFORGE_API_KEY"
export LITEFORGE_BASE_URL="$LITEFORGE_BASE_URL"
export LITEFORGE_DEFAULT_MODEL="$LITEFORGE_DEFAULT_MODEL"
export LITEFORGE_KNOWLEDGE_URL="${LITEFORGE_KNOWLEDGE_URL:-}"
export LITEFORGE_TEMPORAL_URL="${LITEFORGE_TEMPORAL_URL:-}"

# Add LiteForge binaries to PATH
export PATH="\$HOME/.forge/bin:\$HOME/.forge/node/node_modules/.bin:\$PATH"

# LiteForge-scoped CA bundle (only when present and not already set by user).
# These are read by LiteForge and its language bindings only; the bundle is
# never added to the system trust store.
if [ -f "\$HOME/.forge/certs/ca-bundle.crt" ]; then
    [ -z "\$LITEFORGE_EXTRA_CA_FILE" ] && export LITEFORGE_EXTRA_CA_FILE="\$HOME/.forge/certs/ca-bundle.crt"
    [ -z "\$SSL_CERT_FILE" ] && export SSL_CERT_FILE="\$HOME/.forge/certs/ca-bundle.crt"
    [ -z "\$REQUESTS_CA_BUNDLE" ] && export REQUESTS_CA_BUNDLE="\$HOME/.forge/certs/ca-bundle.crt"
    [ -z "\$NODE_EXTRA_CA_CERTS" ] && export NODE_EXTRA_CA_CERTS="\$HOME/.forge/certs/ca-bundle.crt"
fi
EOF

    success "Created $LITEFORGE_HOME/env"

    # Also store API key in keyring as a secondary mechanism
    if [[ -x "$LITEFORGE_HOME/bin/forge" ]]; then
        echo "$LITEFORGE_API_KEY" | "$LITEFORGE_HOME/bin/forge" config set-secret forge-api-key 2>/dev/null || true
    fi

    # Create subdirectories
    mkdir -p "$LITEFORGE_HOME"/{agents,skills,mcp,tools}
    success "Created LiteForge directories"
}

# ============================================================================
# Shell Profile Integration
# ============================================================================

setup_shell_profile() {
    info "Setting up shell profile..."

    local shell_name
    shell_name=$(basename "$SHELL")

    local profile_file
    case "$shell_name" in
        bash)
            if [[ -f "$HOME/.bash_profile" ]]; then
                profile_file="$HOME/.bash_profile"
            else
                profile_file="$HOME/.bashrc"
            fi
            ;;
        zsh)
            profile_file="$HOME/.zshrc"
            ;;
        fish)
            profile_file="$HOME/.config/fish/config.fish"
            ;;
        *)
            warn "Unknown shell: $shell_name"
            echo "  Add the following to your shell profile:"
            echo "    source \"\$HOME/.forge/env\""
            return 0
            ;;
    esac

    local source_line
    if [[ "$shell_name" == "fish" ]]; then
        source_line="source \"\$HOME/.forge/env.fish\""
        # Create fish-compatible env file (unquoted EOF so bash expands $LITEFORGE_BASE_URL etc.)
        cat > "$LITEFORGE_HOME/env.fish" << EOF
# LiteForge Environment Variables for fish shell
# Generated by install.sh on $(date -Iseconds)

# Configuration
set -gx LITEFORGE_API_KEY "$LITEFORGE_API_KEY"
set -gx LITEFORGE_BASE_URL "$LITEFORGE_BASE_URL"
set -gx LITEFORGE_DEFAULT_MODEL "$LITEFORGE_DEFAULT_MODEL"

# Add LiteForge binaries to PATH
fish_add_path --prepend "\$HOME/.forge/bin"
if test -d "\$HOME/.forge/node/node_modules/.bin"
    fish_add_path --prepend "\$HOME/.forge/node/node_modules/.bin"
end

# SSL certificate configuration (only if not already set by user)
if not set -q SSL_CERT_FILE
    set -gx SSL_CERT_FILE "\$HOME/.forge/certs/ca-bundle.crt"
end
if not set -q REQUESTS_CA_BUNDLE
    set -gx REQUESTS_CA_BUNDLE "\$HOME/.forge/certs/ca-bundle.crt"
end
if not set -q NODE_EXTRA_CA_CERTS
    set -gx NODE_EXTRA_CA_CERTS "\$HOME/.forge/certs/ca-bundle.crt"
end
EOF
    else
        source_line="source \"\$HOME/.forge/env\""
    fi

    # Check if already added
    if grep -q "source.*\.forge/env" "$profile_file" 2>/dev/null; then
        success "Shell profile already configured"
        return 0
    fi

    if confirm "Add LiteForge to $profile_file?" "y"; then
        echo "" >> "$profile_file"
        echo "# LiteForge" >> "$profile_file"
        echo "$source_line" >> "$profile_file"
        success "Added LiteForge to $profile_file"
        echo
        warn "Run 'source $profile_file' or restart your shell to apply changes"
    else
        echo
        echo "  Add the following to your shell profile:"
        echo "    $source_line"
    fi
}

# ============================================================================
# Verification
# ============================================================================

verify_installation() {
    info "Verifying installation..."

    local success_count=0
    local total_count=0

    if $INSTALL_CLI; then
        ((total_count++))
        if [[ -x "$LITEFORGE_HOME/bin/forge" ]]; then
            local version
            version=$("$LITEFORGE_HOME/bin/forge" --version 2>/dev/null || echo "unknown")
            success "forge-cli: $version"
            ((success_count++))
        else
            error "forge-cli: not found"
        fi
    fi

    if $INSTALL_PY; then
        ((total_count++))
        if python3 -c "import liteforge" 2>/dev/null; then
            success "liteforge-py: installed"
            ((success_count++))
        else
            error "liteforge-py: import failed"
        fi
    fi

    if $INSTALL_JS; then
        ((total_count++))
        if npm list -g liteforge >/dev/null 2>&1; then
            success "liteforge-js: installed"
            ((success_count++))
        else
            error "liteforge-js: not found"
        fi
    fi

    echo
    if [[ $success_count -eq $total_count ]]; then
        success "All components installed successfully!"
    else
        warn "$success_count/$total_count components installed"
    fi
}

# ============================================================================
# Summary
# ============================================================================

print_summary() {
    echo
    echo -e "${GREEN}${BOLD}Installation Complete!${NC}"
    echo
    echo "  LiteForge Home:    $LITEFORGE_HOME"
    echo "  Config:      $LITEFORGE_HOME/config.toml"
    echo "  Environment: $LITEFORGE_HOME/env"

    if $INSTALL_CLI; then
        echo "  CLI:         $LITEFORGE_HOME/bin/forge"
    fi

    echo
    echo "Next steps:"
    echo "  1. Restart your shell or run: source ~/.forge/env"
    echo "  2. Test the CLI: forge --help"
    echo "  3. Try a chat: forge chat \"Hello, world!\""
    echo
    echo "Documentation: https://github.com/seanpoyner/liteforge"
    echo
}

# ============================================================================
# Main
# ============================================================================

main() {
    echo
    echo -e "${BOLD}LiteForge Installer${NC}"
    echo -e "${DIM}Version: $LITEFORGE_VERSION${NC}"
    echo

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --version)
                LITEFORGE_VERSION="$2"
                shift 2
                ;;
            --home)
                LITEFORGE_HOME="$2"
                shift 2
                ;;
            --with-ca-bundle)
                WITH_CERTS=1
                shift
                ;;
            --skip-certs)
                # Deprecated: CA bundle install is now opt-in (off by default).
                # Accepted as a no-op for backward compatibility.
                shift
                ;;
            --build-from-source)
                LITEFORGE_BUILD_FROM_SOURCE=1
                shift
                ;;
            --non-interactive)
                NON_INTERACTIVE=1
                shift
                ;;
            -h|--help)
                echo "Usage: install.sh [OPTIONS]"
                echo
                echo "Options:"
                echo "  --version VERSION      Install specific version (default: latest)"
                echo "  --home PATH            Install to custom directory (default: ~/.forge)"
                echo "  --with-ca-bundle       Install a LiteForge-scoped CA bundle (for internal/proxy CAs;"
                echo "                         off by default, never touches the system trust store)"
                echo "  --build-from-source    Force building from source instead of downloading pre-built binaries"
                echo "  --non-interactive      Run without prompts (requires env vars)"
                echo "  -h, --help             Show this help message"
                echo
                echo "Environment variables:"
                echo "  LITEFORGE_API_KEY              API key (required)"
                echo "  LITEFORGE_BASE_URL             Base URL (optional)"
                echo "  LITEFORGE_DEFAULT_MODEL        Default model (optional)"
                echo "  LITEFORGE_BUILD_FROM_SOURCE    Set to 1 to force source build (same as --build-from-source)"
                exit 0
                ;;
            *)
                die "Unknown option: $1"
                ;;
        esac
    done

    detect_platform
    check_prerequisites

    # CA bundle install is opt-in (off by default). Most users do not need it;
    # it only matters behind a TLS-inspecting proxy with an internal CA. When
    # enabled it is scoped to LiteForge, never added to the system trust store.
    if [[ -n "${WITH_CERTS:-}" ]]; then
        install_ca_certs
    fi

    collect_configuration
    validate_credentials
    select_components

    install_cli
    install_python_sdk
    install_node_sdk
    install_rust_sdk

    write_configuration
    setup_shell_profile
    verify_installation
    print_summary
}

main "$@"
