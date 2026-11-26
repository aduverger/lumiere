# Contributing & Testing

Lumiere aims for **beauty by simplicity**: clear primitives, direct control flow, zero `unsafe` code, and fast, durable tests.

## Local development workflow

```bash
# Check formatting (configured in rustfmt.toml)
cargo fmt --all -- --check

# Lint with warnings denied across both feature sets
cargo clippy --all-targets -- -D warnings
cargo clippy --no-default-features --all-targets -- -D warnings

# Run the full test suite (Git + Jujutsu, and Git-only)
cargo test
cargo test --no-default-features

# Build an optimized release binary (LTO + single codegen unit + stripped symbols)
cargo build --release
```

## Testing philosophy

Lumiere's tests focus on durable user-visible behavior and real invariants rather than internal implementation details:

- **Isolated temporary repositories:** VCS, diff loading, watcher, workspace, and thread-store tests use `tempfile::TempDir` and pass explicit paths instead of mutating global working directories.
- **Headless TUI rendering with `ratatui::backend::TestBackend`:** Diff rendering (side-by-side, added/deleted single-panel, binary, hunks-only folding, inline thread overlays, stacked headers, narrow footers, and Thread View modals) is tested directly against rendered terminal cell buffers.
- **Headless event-loop tests:** `App::handle_event` returns `LoopAction` and operates cleanly without a TTY, letting tests drive multi-step keystroke and mouse workflows (commenting, queueing, global search jumping into hidden lines, async `AgentWorker` picker updates, and scroll coalescing).
- **Concurrency & UTF-8 regression coverage:** Multi-threaded `ThreadStore` lock/rename stress tests, Unicode char-boundary tests for search/selection/truncation, and `serde` GraphQL fixture tests protect subtle edge cases.
- **Jujutsu integration tests:** Tests requiring the external `jj` binary automatically skip when `jj` is not installed in `PATH`.

## Previewing the documentation website

This documentation site is built with [mdBook](https://rust-lang.github.io/mdBook/).

```bash
# Install mdBook once
cargo install mdbook

# Serve locally with live reload
mdbook serve docs --open

# Build static HTML into docs/book/
mdbook build docs
```

Merging changes to `docs/**` on `main` automatically builds and deploys the site to GitHub Pages via `.github/workflows/docs.yml`.

## Continuous Integration & Releases

- **`.github/workflows/ci.yml`** runs `cargo fmt --check`, `cargo clippy -- -D warnings` (default and `--no-default-features`), `cargo test` on both `ubuntu-latest` and `macos-latest`, and an MSRV check on Rust `1.89`.
- **`.github/workflows/dependency-review.yml`** runs GitHub Dependency Review on pull requests (failing on moderate or higher advisories).
- **`.github/workflows/security-audit.yml`** runs a daily scheduled RustSec audit (`cargo-audit`) across all dependencies in `Cargo.lock` and opens issues on new advisories.
- **`.github/workflows/lint-pr-title.yml`** enforces Conventional Commit PR titles so Release Drafter can classify changes automatically.
- **`.github/workflows/docs.yml`** builds the mdBook site on PRs and deploys to GitHub Pages on `main`.
- **`.github/workflows/release-drafter.yml`** (configured in `.github/release-drafter.yml`) auto-labels PRs (`breaking`, `feature`, `fix`, `chore`) and maintains a draft GitHub Release on `main` with `0.x` semantic versioning (`breaking` and `feat` bump minor; `fix` and `chore` bump patch).
- **`.github/workflows/release.yml`** triggers when a GitHub Release is published (or via `workflow_dispatch` with a release tag), builds Linux (`x86_64`) and macOS (`aarch64`, `x86_64`) binaries with `LUMIERE_VERSION` stamped from the tag, and uploads checksummed tarballs to that release.
