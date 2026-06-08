# Contributing

Thanks for helping improve LiteForge! This page covers local development, the release flow, and how
to edit this wiki.

## Development setup

```bash
git clone https://github.com/seanpoyner/liteforge.git
cd liteforge

# C toolchain (Debian/Ubuntu)
sudo apt-get install -y build-essential pkg-config libssl-dev

cargo build --workspace
cargo test  --workspace
```

### Building the pieces

```bash
cargo build --release -p liteforge     # core lib
cargo build --release -p forge-cli     # forge binary → target/release/forge

# Python wheel (from the binding crate)
cd crates/liteforge-py && maturin build --release

# JS native addon
bash scripts/build-js.sh
```

## Conventions

- **Rust 2021**, async via tokio. Library code uses `async fn`; never `unwrap()` in library code —
  use `?` and `ForgeError`.
- Add dependencies to the workspace `Cargo.toml` `[workspace.dependencies]` and reference them with
  `{ workspace = true }`.
- Keep `default = []`; gate optional integrations (like `otel`) behind features.
- Colocate unit tests (`#[cfg(test)] mod tests`); integration tests go in `tests/`.
- Match the style of the surrounding code.

## Pull requests

1. Branch off `main`.
2. Make the change with tests; run `cargo test --workspace` and `cargo fmt`.
3. Use clear, conventional commit messages.
4. Open the PR against `main` with a short description of what and why.

## Release flow

Releases are tag‑driven. A version bump is a commit touching the inherited `version` and the binding
manifests, followed by a `v*` tag:

- `Cargo.toml` + `Cargo.lock` (all workspace crates share one version)
- `crates/liteforge-py/pyproject.toml`
- `crates/liteforge-js/package.json`

Tagging `vX.Y.Z` triggers the GitHub Actions release workflow, which builds CLI binaries (4
platforms), Python wheels, the JS addon, publishes to **crates.io**, **PyPI**, and **npm**, and
attaches a `SHA256SUMS` manifest used by the installers' fail‑closed verification.

> Do **not** commit `dist/` or build artifacts — stale wheels there get bundled into release jobs and
> cause duplicate‑file publish failures.

## Editing this wiki

The wiki's source of truth is the [`wiki/`](https://github.com/seanpoyner/liteforge/tree/main/wiki)
directory in the main repo — **not** the GitHub wiki UI. This keeps wiki changes in version control
and under PR review.

```bash
# 1. Edit the page(s) under wiki/
$EDITOR wiki/Quickstart.md

# 2. Commit with the rest of your change (normal PR flow)

# 3. Publish to the GitHub wiki
scripts/sync-wiki.sh
```

`scripts/sync-wiki.sh` copies `wiki/*.md` into the wiki repo
(`github.com/seanpoyner/liteforge.wiki.git`) and pushes. It authenticates via the `gh` credential
helper, so make sure you're logged in (`gh auth status`); it never embeds a token.

### Conventions

- One page per file; the filename (minus `.md`) is the page slug. `Tools-and-Agents.md` →
  `/wiki/Tools-and-Agents`.
- Special pages: `Home.md`, `_Sidebar.md`, `_Footer.md`.
- Link between pages by slug: `[Quickstart](Quickstart)`.
- Diagrams are **Mermaid** fenced blocks (GitHub renders them natively — no images to export).
- This wiki is a guided tour; link out to [docs.rs](https://docs.rs/liteforge) and the repo
  [`docs/`](https://github.com/seanpoyner/liteforge/tree/main/docs) tree for deep API reference
  rather than duplicating it.

### One‑time wiki initialization

A brand‑new GitHub wiki repo doesn't exist until the first page is created. If `sync-wiki.sh` fails
with *“Repository not found”*, create any page once via the web UI at
`github.com/seanpoyner/liteforge/wiki` (click **Create the first page** → Save), then re‑run the
script. After that, the script owns all subsequent updates.

## Code of conduct

Be kind and constructive. Assume good faith. File issues and PRs that you'd want to receive.
