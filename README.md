<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-wordmark.svg" />
    <source media="(prefers-color-scheme: light)" srcset="assets/light/logo-wordmark.svg" />
    <img src="assets/light/logo-wordmark.svg" width="360" alt="Lumiere" />
  </picture>
</p>

<p align="center">
  <strong>Review diffs in the terminal and discuss them live with your agent.</strong>
</p>

<p align="center">
  <a href="https://lumiere.computer">Website & Docs</a> ·
  <a href="#install">Install</a> ·
  <a href="#quick-start">Quick Start</a> ·
  <a href="#comparison">Comparison</a> ·
  <a href="#going-deeper">Reference</a> ·
  <a href="#development">Development</a>
</p>

<p align="center">
  <a href="https://github.com/aduverger/lumiere/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/aduverger/lumiere/ci.yml?branch=main&style=flat-square&label=CI" alt="CI" /></a>
  <a href="https://github.com/aduverger/lumiere/releases"><img src="https://img.shields.io/github/v/release/aduverger/lumiere?style=flat-square" alt="Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT License" /></a>
  <img src="https://img.shields.io/badge/rust-1.97.1%2B-orange?style=flat-square" alt="Rust 1.97.1+" />
</p>

Lumiere is a fast, side-by-side terminal diff viewer written in Rust for reviewing code alongside a coding agent. Open uncommitted work, a feature branch, a commit range, a GitHub PR, or a whole folder of repositories — comment directly on hunks or line selections, and watch your agent answer inside the same inline thread without leaving the review.

<table>
  <tr>
    <td width="50%" align="center">
      <img src="assets/screenshots/hunks-only.png" alt="Hunks-only side-by-side diff folding unchanged lines" />
      <br />
      <sub>Hunks-only view by default — press <code>z</code> to unfold the full file.</sub>
    </td>
    <td width="50%" align="center">
      <img src="assets/screenshots/thread.png" alt="Inline review thread with live agent reply" />
      <br />
      <sub>Comment on a selection, hunk, or file — your agent replies live in the same thread.</sub>
    </td>
  </tr>
</table>

## How it works

1. **Open any diff** — working tree changes, a feature branch (`origin/main...-`), a commit range, a GitHub PR (`--pr 123`), or a multi-repo workspace directory.
2. **Comment right on the code** — focus a hunk (`[` / `]`) or drag with the mouse, press `i`, and hit `Enter` to send immediately (or `Alt+Enter` / `Ctrl+S` to queue a batch and send together with `s`).
3. **Iterate inside the thread** — Lumiere dispatches the exact file, lines, code context, and reply command to your coding agent. When the agent answers with `lumiere thread reply`, its Markdown-formatted response appears live in the open TUI.

### Highlights

- **Two-way review threads, not one-way notes** — Inline threads anchored to selections, hunks, or files with Markdown + syntax-highlighted code blocks, queued batches, resolve/reopen tracking, and a focused modal for long discussions.
- **Hunks-only by default, full file one key away** — Opens focused on changes with three context lines around each hunk (like GitHub PR reviews); press `z` to switch to full files while keeping your hunk in place.
- **Multi-repository workspaces** — Run `lumiere diff` in a parent folder containing multiple repositories (`api/`, `web/`, …), switch with `R`, and dispatch queued feedback across every repository in one message.
- **GitHub PRs & stacked commits** — Review PRs via `--pr` / `--detect-pr` with GitHub viewed-file sync, or step commit-by-commit through a range with `--stacked`.
- **Git & Jujutsu native, built for speed** — Native Git and Jujutsu backends with Tree-sitter syntax highlighting, ~25 ms startup even on large repositories, and 0% idle watch CPU.
- **`pi` coding agent integration** — Includes a [`/lumiere`](pi-extension/) extension for [pi](https://github.com/earendil-works/pi).

## Install

### Install script (macOS & Linux)

```bash
curl -fsSL https://lumiere.computer/install.sh | sh
```

### Homebrew (macOS & Linux)

```bash
brew install aduverger/tap/lumiere
```

### Cargo (Rust 1.97.1+)

```bash
cargo install lumiere-cli --locked
```

See the [installation guide](https://lumiere.computer/getting-started/installation.html) for `cargo-binstall`, `mise`, Nix, pre-built release archives, custom install paths, and building without Jujutsu (`--no-default-features`).

## Quick start

```bash
lumiere diff                    # uncommitted changes
lumiere diff origin/main...-    # everything this branch adds, uncommitted changes included
lumiere diff HEAD~3..HEAD       # a range of commits
lumiere diff --pr 123           # a GitHub pull request (uses gh CLI)
lumiere diff -w                 # watch mode: auto-reload on file changes
```

Inside the viewer: press `[` / `]` to jump between hunks, `i` to comment, `Enter` to send to the attached agent, `z` to toggle full file, `A` to pick an agent, and `?` for every shortcut.

## Comparison

How Lumiere compares to other interactive terminal code-review TUIs on **features** and **measured performance** (see the [full comparison guide](docs/src/comparison.md) for `difftastic`, `delta`, `diff-so-fancy`, GNU `diff`, methodology, and complete metrics):

### Features

| Capability | **Lumiere** | [hunk](https://github.com/modem-dev/hunk) | [plannotator-tui](https://github.com/plannotator/plannotator-tui) | [lumen](https://github.com/jnsahaj/lumen) |
| --- | :---: | :---: | :---: | :---: |
| Live two-way agent threads in TUI | ✅ | ❌ | ❌ | ❌ |
| Multi-repo workspace review | ✅ | ❌ | ❌ | ❌ |
| Inline review comments / notes | ✅ | ✅ | ✅ | ✅ |
| GitHub PR review | ✅ | ✅ | ✅ | ✅ |
| Watch mode (live auto-reload) | ✅ | ✅ | ❌ | ✅ |
| Git & Jujutsu (`jj`) support | ✅ | ✅¹ | Git only | ✅ |
| Hunks-only & full-file toggle | ✅ | ✅ | ✅ | ❌ |
| Split & unified / auto layouts | Side-by-side² | ✅ | ✅ | Side-by-side |
| Git pager / stdin patch mode | ❌ | ✅ | ❌ | ❌ |
| Markdown / plan review & gate | ❌ | ❌ | ✅ | ❌ |

<sup>¹ `hunk` also supports Sapling (`sl`). ² Lumiere renders side-by-side and lets you focus one panel (`0` old, `-` new, `=` both).</sup>

### Performance

Median of 5 runs on Linux (`aarch64`, 2 vCPU) using [`bench/bench.py`](bench/bench.py) against official release binaries (`lumiere 0.1.0`, `hunk 0.25.0`, `plannotator-tui 0.12.0`, `lumen 2.32.0`):

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/benchmarks-dark.svg" />
    <source media="(prefers-color-scheme: light)" srcset="assets/benchmarks-light.svg" />
    <img src="assets/benchmarks-light.svg" width="840" alt="Lumiere benchmark comparison chart against plannotator-tui, hunk, and lumen" />
  </picture>
</p>

<sub>Lumiere also hits <strong>0.0% idle watch CPU</strong>, opens the 360k-line file in <strong>166.5 ms</strong> (33.9× faster than lumen; skipped by hunk), and toggles full-file view in <strong>110.2 ms</strong> (capped at 5 MiB in plannotator-tui). See the full 21-metric breakdown and reproduction steps in <a href="bench/README.md#measured-results">bench/README.md</a> and <a href="docs/src/comparison.md">docs/src/comparison.md</a>.</sub>

---

## Going deeper

Full guides and architecture notes live at **[lumiere.computer](https://lumiere.computer)**. Below is a quick reference for everyday use.

### What to review

| Reference | Shows |
| --- | --- |
| *(none)* | Uncommitted changes |
| `SHA` | One commit |
| `A..B` | From `A` to `B` |
| `A...B` | From the merge base of `A` and `B` to `B` |
| `A..-` | From `A` to the working tree |
| `A...-` | From the merge base of `A` and `HEAD` to the working tree |

`A...-` is the form to use on a feature branch: it keeps showing only your branch's changes even after `A` moves forward.

| Option | Effect |
| --- | --- |
| `--watch`, `-w` | Reload automatically when files change |
| `--file PATH`, `-f` | Show only matching files |
| `--focus PATH` | Open focused on this file |
| `--stacked` | Step through a commit range one commit at a time (`Ctrl+L` / `Ctrl+H`) |
| `--pr N`, `--detect-pr` | Review a GitHub PR, or auto-detect the current branch's PR |
| `--wrap` | Soft-wrap long diff lines |
| `--view hunks\|full` | Start in hunks-only (default) or full-file view |
| `--theme NAME`, `-t` | Override the colour theme |
| `--agent-target TARGET` | Target pane/terminal ID of the coding agent (alias: `--agent-block`) |
| `--vcs git\|jj` | Force the VCS backend (passed before `diff`) |

`lumiere diff` works from any subdirectory of a repository. On Linux, `--watch` skips gitignored directories such as `node_modules` and `.venv`. Files over 1 MB use lightweight line-by-line highlighting to stay responsive.

<p align="center">
  <img src="assets/screenshots/diff.png" width="760" alt="Full-file side-by-side diff view with file tree sidebar" />
  <br />
  <sub>Full-file view (<code>z</code>) with file tree sidebar, syntax highlighting, and inline word diffs.</sub>
</p>

### Review threads & coding agents

Select lines with the mouse, or focus a hunk with `[` / `]`; with neither, `i` opens a file-level thread. Hunk comments anchor to the hunk's new lines (or old lines when only the old panel is shown with `0`).

| Key | Effect |
| --- | --- |
| `i` | Start a thread, reply to an open thread, or edit an unsent comment |
| `Enter` | Save the comment and send it to the attached agent now |
| `Option+Enter`, `Ctrl+S` | Save the comment as queued |
| `Shift+Enter`, `Ctrl+J` | Insert a new line |
| `s` | Send all queued comments together as one message |
| `x` | Resolve or reopen a thread (replying to a resolved thread reopens it) |
| `I` | Open the threads list (`x` resolves, `r` toggles resolved threads) |
| `A` | Pick which coding agent to attach to (or `none`) |

<p align="center">
  <img src="assets/screenshots/agents.png" width="760" alt="Agents picker modal showing running coding agents and live status" />
  <br />
  <sub>Press <code>A</code> to attach to any coding agent running in the repository or workspace.</sub>
</p>

- **Agent discovery:** Lumiere discovers coding agents running in [Rex](https://rex.dev) panes reporting [Program Status](https://www.superlogical.com/rex/docs/build/program-status) (such as pi, Claude Code, Codex 0.163+, and Antigravity). It auto-attaches to the most recently active agent whose working directory matches the reviewed repo or workspace; `--agent-target TARGET` pins one explicitly.
- **Safety when the agent is waiting for input:** If the agent is waiting on a confirmation prompt, Lumiere holds comments in the queue instead of submitting into its prompt. Answer the agent, then press `s`.
- **Without an agent:** `Enter` queues comments, and `s` exits and prints the queued review to stdout.
- **Storage & outdated anchors:** Threads persist per repository outside the repo (`~/.local/state/lumiere/` on Linux) across every range you review. When files change, threads follow their lines if the anchored text moves cleanly; otherwise they are marked `outdated`.

Agents read and reply to threads with three subcommands:

```bash
lumiere thread list [--awaiting] [--json] [--repo PATH]
lumiere thread show <thread-id> [--json] [--repo PATH]
lumiere thread reply <thread-id> (--body TEXT | --stdin) [--repo PATH] [--author NAME]
```

### Multi-repository workspaces

Run `lumiere diff` from a parent folder whose immediate subdirectories are repositories:

```bash
cd ~/code/my-workspace      # contains api/, web/, ...
lumiere diff origin/main...-
```

- Opens the first repository (alphabetically) that has changes; press `R` to switch repositories.
- Each repository keeps its own threads, viewed markers, and scroll position.
- Pressing `s` sends queued comments across every opened repository in one message.
- Symlinks and plain non-repo folders are ignored; `--pr`, `--detect-pr`, and `--stacked` require a single repository.

### `pi` command (`/lumiere`)

[`pi-extension/`](pi-extension/) adds `/lumiere` to [pi](https://github.com/earendil-works/pi). Symlink it once from a checkout and run `/reload` in pi:

```bash
ln -s "$(pwd)/pi-extension" ~/.pi/agent/extensions/lumiere
```

```text
/lumiere                    review origin/main...- with watch mode on
/lumiere diff HEAD~3..-     pass any lumiere arguments
```

- **Inside Rex:** opens Lumiere in a zoomed pane beside pi, pre-attached to that session while pi keeps running.
- **Elsewhere:** suspends pi while Lumiere owns the terminal; press `s` and `Enter` to send comments back, or `q` to cancel.

Set `LUMIERE_BIN` if `lumiere` is not on `PATH`. The extension and binary should come from the same version.

### Keybindings & settings

Press `?` inside Lumiere for the full keybinding sheet.

<details>
<summary>Common keybindings</summary>

| Key | Effect |
| --- | --- |
| `j` / `k`, arrows | Scroll, or move in a list |
| `Option+Up` / `Option+Down`, `Option+J` / `Option+K` | Previous / next file |
| `[` / `]` | Previous / next hunk |
| `Ctrl+P` | File picker |
| `/`, `Ctrl+F` | Search in current file, or across every file |
| `Tab` | Toggle the file sidebar |
| `Space` | Mark file as viewed and advance |
| `0` / `-` / `=` | Old side only, new side only, both |
| `w` | Toggle watch mode |
| `z` | Toggle hunks-only / full file |
| `e` | Open current file in `EDITOR` |
| `y` | Copy selection, or current file path |
| `R` | Switch repository in a workspace |
| `q` | Quit |

`Option` is `Alt` on Linux.

</details>

Lumiere reads `~/.config/lumiere/lumiere.config.json` (or `--config PATH` passed before `diff`):

```json
{
  "theme": "catppuccin-mocha",
  "wrap": true,
  "view": "hunks"
}
```

Theme precedence: `--theme` → config file → `LUMIERE_THEME` → terminal background auto-detection.

Supported themes: `dark`, `light`, `catppuccin-mocha`, `catppuccin-latte`, `dracula`, `nord`, `one-dark`, `gruvbox-dark`, `gruvbox-light`, `solarized-dark`, `solarized-light`, `flexoki-dark`, `flexoki-light`.

### Relation to Lumen

Lumiere started as a fork of [Lumen](https://github.com/jnsahaj/lumen) v2.32.0 by Sahaj Jain and keeps its side-by-side diff foundation. It is now developed independently and restarts versioning at `0.1.0`:

- Replaces Lumen's one-way annotations with live two-way review threads backed by a coding agent.
- Adds hunks-only view by default, multi-repository workspace reviews, the `A...-` reference, and the `pi` extension.
- Removes Lumen's standalone LLM subcommands (`draft`, `explain`, `operate`, `list`, `configure`).
- Uses `lumiere`, `lumiere.config.json`, and `LUMIERE_THEME` (Lumen's config and env vars are not read).

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --no-default-features
```

Lumiere is organized around five core primitives (`vcs`, `diff`, `review`, `search`, `ui`) documented in [`docs/`](docs/src/architecture/overview.md). Preview the documentation site locally with [mdBook](https://rust-lang.github.io/mdBook/):

```bash
mdbook serve docs --open
```

`bench/` measures startup, memory, navigation, and watch latency across changes — see [bench/README.md](bench/README.md).

## License

MIT. Copyright Sahaj Jain (Lumen) and Alexandre Duverger. See [LICENSE](LICENSE).
