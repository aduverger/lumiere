<p align="center"><img src="assets/logo.svg" width="104" alt="Lumiere logo" /></p>

<h1 align="center">Lumiere</h1>

<p align="center">
  <strong>Review diffs in the terminal and discuss them live with your coding agent.</strong>
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
  <img src="https://img.shields.io/badge/rust-1.89%2B-orange?style=flat-square" alt="Rust 1.89+" />
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
- **Git & Jujutsu native, built for speed** — Uses vendored `libgit2` and `jj-lib` directly, lazy per-language Tree-sitter highlighting, cached diff metrics, and event-driven redraws (~22 ms startup, ~0% idle watch CPU).
- **`pi` coding agent integration** — Includes a [`/lumiere`](pi-extension/) extension for [pi](https://github.com/earendil-works/pi).

## Install

### Pre-built binaries

Download a `.tar.gz` archive (Linux `x86_64`, macOS `aarch64` / `x86_64`) with its `.sha256` checksum from [GitHub Releases](https://github.com/aduverger/lumiere/releases).

### Cargo (Rust 1.89+)

```bash
cargo install --git https://github.com/aduverger/lumiere --locked
```

From a local checkout: `cargo install --path . --locked` (add `--no-default-features` to build Git-only without `jj-lib`).

### Nix flake

```bash
nix run github:aduverger/lumiere -- diff
# or install into your profile:
nix profile install github:aduverger/lumiere
```

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

How Lumiere compares to other terminal diff viewers and review tools on **features** and **measured performance** (see the [full comparison guide](docs/src/comparison.md) for methodology, complete metrics, and when to pick each tool):

### Features

| Capability | **Lumiere** | [hunk](https://github.com/modem-dev/hunk) | [plannotator-tui](https://github.com/plannotator/plannotator-tui) | [diffs-rs](https://github.com/plannotator/diffs-rs) | [lumen](https://github.com/jnsahaj/lumen) | [difftastic](https://github.com/Wilfred/difftastic) | [delta](https://github.com/dandavison/delta) | [diff-so-fancy](https://github.com/so-fancy/diff-so-fancy) | [diff](https://www.gnu.org/software/diffutils/) |
| --- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| Review-first interactive TUI | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | Partial¹ | ❌ | ❌ |
| Multi-file sidebar & tree | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Inline review comments / notes | ✅ | ✅ | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Live two-way agent threads in TUI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Multi-repo workspace review | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| GitHub PR review | ✅ | ✅ | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Watch mode (live auto-reload) | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Git & Jujutsu (`jj`) support | ✅ | ✅² | Git only | Git / patch | ✅ | Any | Any | Any | Any |
| Hunks-only & full-file toggle | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Split & unified / auto layouts | Side-by-side³ | ✅ | ✅ | ✅ | Side-by-side | Split / inline | ✅ | Unified | Unified |
| Mouse support | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Syntax highlighting | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Structural (AST) diffing | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |
| Git pager / stdin patch mode | ❌ | ✅ | ❌ | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| Markdown / plan review & gate | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Embeddable UI crate / component | ❌ | OpenTUI | ❌ | Ratatui | ❌ | ❌ | ❌ | ❌ | ❌ |

<sup>¹ `delta` is primarily a Git pager, with an optional interactive `--tui` mode via `gitu`. ² `hunk` also supports Sapling (`sl`). ³ Lumiere renders side-by-side and lets you focus one panel (`0` old, `-` new, `=` both).</sup>

### Performance (`bench/bench.py`)

Median of 5 runs on Linux (`aarch64`, 2 vCPU) using [`bench/bench.py`](bench/bench.py) against official release binaries (`lumiere 0.1.0`, `plannotator-tui 0.12.0` backed by `diffs-rs`, `hunk 0.25.0`, `lumen 2.32.0`):

| Metric | **Lumiere** | **plannotator-tui** (`diffs-rs`) | **hunk** | **lumen** |
| --- | ---: | ---: | ---: | ---: |
| Start (`small`, 3 files) | **21.7 ms** | 51.2 ms | 481.5 ms | 577.9 ms |
| Peak RAM (`small`) | **8.6 MB** | 16.5 MB | 211.9 MB | 41.1 MB |
| Start (`many`, 300 files) | **65.9 ms** | 239.8 ms | 851.5 ms | 656.0 ms |
| First key / next file (`many`) | 5.7 / 4.4 ms | **2.4 / 1.0 ms** | 85.9 / 14.2 ms | 5.5 / 5.8 ms |
| Start (`large`, 360k-line / 5.3 MiB file) | 199.8 ms | **151.8 ms** | 516.4 ms | 755.5 ms |
| Open 360k-line file / whole-file view | **210.8 / 185.4 ms** | *streamed / capped¹* | *skipped²* | 5,590 ms / *n/a* |
| Peak RAM (`large`) | 128.7 MB | **16.7 MB¹** | 197.6 MB² | 1,109.5 MB |
| Start (`ignored`, 8k `.venv` dirs, `--watch`) | **21.4 ms** | *no watch* | 515.1 ms | 1,004.5 ms |
| Idle CPU (`--watch`, 4 s) | **0.0%** | *no watch* | 0.3% | 1.5%³ |

<sup>¹ `plannotator-tui` streams only patch hunks (very low RAM) and caps whole-file view at 5 MiB. ² `hunk` prints "File too large to render automatically." on the 5.3 MiB file. ³ `lumen 2.32.0` reloads continuously on `Access` events in watch mode. Run `bench/bench.py` yourself to reproduce on your hardware — full 13-metric table in [docs/src/comparison.md](docs/src/comparison.md).</sup>

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

`lumiere diff` works from any subdirectory of a repository. On Linux, `--watch` skips gitignored directories such as `node_modules` and `.venv`. Files over 1 MB are highlighted line-by-line (skipping whole-file Tree-sitter parsing and sticky enclosing-scope headers) to stay fast.

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

- **Agent discovery:** Lumiere connects to coding agents through a host-agnostic `AgentHost` layer (currently backed by [Rex](https://rex.dev) panes reporting [Program Status](https://www.superlogical.com/rex/docs/build/program-status), such as pi, Claude Code, Codex 0.163+, and Antigravity). It auto-attaches to the most recently active agent whose working directory matches the reviewed repo or workspace; `--agent-target TARGET` pins one explicitly.
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
