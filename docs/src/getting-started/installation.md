# Installation

Lumiere targets **Linux** and **macOS** and requires **Rust 1.97.1 or later** when building from source.

<div class="install-tabs" data-install-tabs>
  <div class="install-tabs-bar" role="tablist" aria-label="Install method">
    <button type="button" class="install-tab" role="tab" aria-selected="true" data-tab="curl" data-cmd="curl -fsSL https://lumiere.computer/install.sh | sh">curl</button>
    <button type="button" class="install-tab" role="tab" aria-selected="false" data-tab="homebrew" data-cmd="brew install aduverger/tap/lumiere">homebrew</button>
    <button type="button" class="install-tab" role="tab" aria-selected="false" data-tab="cargo" data-cmd="cargo install lumiere-cli --locked">cargo</button>
    <button type="button" class="install-tab" role="tab" aria-selected="false" data-tab="mise" data-cmd="mise use -g github:aduverger/lumiere">mise</button>
    <button type="button" class="install-tab" role="tab" aria-selected="false" data-tab="nix" data-cmd="nix profile install github:aduverger/lumiere">nix</button>
  </div>
  <div class="install-tabs-body">
    <span class="install-tabs-prompt" aria-hidden="true">$</span>
    <code class="install-tabs-cmd">curl -fsSL https://lumiere.computer/install.sh | sh</code>
    <button type="button" class="install-tabs-copy" aria-label="Copy install command" title="Copy">
      <svg class="icon-copy" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="5" y="5" width="9" height="9" rx="1.5"/><path d="M11 5V3.5A1.5 1.5 0 0 0 9.5 2h-6A1.5 1.5 0 0 0 2 3.5v6A1.5 1.5 0 0 0 3.5 11H5"/></svg>
      <svg class="icon-check" viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 8.5 6.5 12 13 4.5"/></svg>
    </button>
  </div>
</div>

## Install script (macOS & Linux)

Download the latest pre-built binary with SHA-256 verification into `~/.local/bin`:

```bash
curl -fsSL https://lumiere.computer/install.sh | sh
```

You can pin a release or choose a different directory with environment variables:

```bash
curl -fsSL https://lumiere.computer/install.sh \
  | LUMIERE_VERSION=v0.1.1 LUMIERE_INSTALL_DIR=/usr/local/bin sh
```

## Homebrew (macOS & Linux)

```bash
brew install aduverger/tap/lumiere
```

## Install with Cargo

Install from [crates.io](https://crates.io/crates/lumiere-cli):

```bash
cargo install lumiere-cli --locked
```

Or download a pre-built binary via [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall):

```bash
cargo binstall lumiere-cli
```

Or from a local clone:

```bash
git clone https://github.com/aduverger/lumiere.git
cd lumiere
cargo install --path . --locked
```

All commands place the `lumiere` binary in `~/.cargo/bin`.

### Building without Jujutsu (`jj-lib`)

Jujutsu support is enabled by default via the `jj` Cargo feature. If you only use Git and want a smaller binary and faster compile time, build with `--no-default-features`:

```bash
cargo install lumiere-cli --locked --no-default-features
```

## mise

Install a pre-built release directly with [mise](https://mise.jdx.dev):

```bash
mise use -g github:aduverger/lumiere
```

## Pre-built binaries

Tagged releases publish pre-built archives and SHA-256 checksums on the [GitHub Releases](https://github.com/aduverger/lumiere/releases) page for:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `aarch64-apple-darwin` (Apple Silicon)
- `x86_64-apple-darwin` (Intel macOS)

## Nix Flake

If you use Nix with flakes enabled:

```bash
nix run github:aduverger/lumiere -- diff
```

Or install it into your profile:

```bash
nix profile install github:aduverger/lumiere
```

## Optional integrations

| Tool | Needed for |
|---|---|
| [GitHub CLI (`gh`)](https://cli.github.com/) | Reviewing GitHub PRs via `--pr` / `--detect-pr` and syncing file viewed checkboxes |
| [Rex](https://rex.dev) | Live two-way review threads with coding agents running in sibling terminal panes |
| [pi](https://github.com/earendil-works/pi) | The `/lumiere` slash command inside pi (see below) |

## Installing the `pi` extension

If you use the [pi](https://github.com/earendil-works/pi) coding agent, symlink `pi-extension/` once from your checkout and run `/reload` inside pi:

```bash
ln -s "$(pwd)/pi-extension" ~/.pi/agent/extensions/lumiere
```

See [Coding Agents & Rex Integration](../guide/coding-agents.md) for how `/lumiere` behaves inside and outside Rex.
