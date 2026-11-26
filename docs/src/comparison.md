# Comparison

Lumiere sits in a growing ecosystem of terminal diff viewers and review tools. This page compares Lumiere with the tools developers most often evaluate alongside it — including interactive review TUIs, embeddable diff engines, and non-interactive Git pagers — and outlines where each tool is the better fit.

## Feature matrix

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

1. **`delta`** is primarily a Git pager, with an optional interactive `--tui` mode powered by `gitu`.
2. **`hunk`** supports Sapling (`sl`) in addition to Git and Jujutsu.
3. **Lumiere** renders side-by-side by default and lets you focus one panel (`0` old, `-` new, `=` both).

## How they differ & when to choose each

### Lumiere
Built specifically for the **live back-and-forth code-review loop with a coding agent** in the terminal. Choose Lumiere if you want:
- **Two-way inline review threads** where you comment on a hunk or selection and your agent replies directly inside the same thread while the review stays open.
- **Multi-repository workspace reviews** (`api/`, `web/`, …) with per-repo state and single-shot feedback dispatch across repos.
- **A fast, self-contained Rust TUI** with hunks-only default view, live watch mode, GitHub PR viewed-file sync, stacked commits, and native `libgit2` + `jj-lib` support.

### [hunk](https://github.com/modem-dev/hunk)
An interactive diff viewer built on OpenTUI and Pierre diffs. Choose Hunk if you want:
- **Git / Jujutsu / Sapling pager mode** (`git config core.pager "hunk pager"`) and stdin patch viewing.
- **Responsive auto split/unified layouts** or an optional **browser review mode** (`hunk diff --web`).
- **TypeScript extensions** or an embeddable OpenTUI component.
- Hunk supports agent-authored review notes (pushed into a live session via a skill), whereas Lumiere focuses on interactive two-way threads initiated by the human reviewer.

### [plannotator-tui](https://github.com/plannotator/plannotator-tui) & [diffs-rs](https://github.com/plannotator/diffs-rs)
Two closely related Rust + Ratatui projects from the Plannotator ecosystem:
- **`plannotator-tui`** shines when you review **Markdown plans, design docs, and agent transcripts** alongside code diffs, want **scriptable approval gates** (`--gate --require-approval`), or need built-in **Herdr**, **Rex**, and **pi** dispatch. Its code review uses one-way notes/suggestions sent to an agent rather than two-way in-thread replies.
- **`diffs-rs`** is the standalone Ratatui diff viewer and **embeddable Rust crate** powering `plannotator-tui`'s diffs. It is ideal if you want progressive streaming on massive patch files (hundreds of megabytes) from stdin or want to embed a Ratatui diff widget inside your own Rust application.

### [lumen](https://github.com/jnsahaj/lumen)
Lumiere's upstream origin (`v2.32.0`). Choose Lumen if you want built-in **LLM CLI subcommands** (`lumen explain`, `lumen draft`, `lumen operate`) across 10+ AI providers alongside a full-file side-by-side diff viewer. Choose Lumiere if you want hunks-only view by default, multi-repo workspaces, and live two-way coding-agent threads instead of standalone LLM commands.

### [difftastic](https://github.com/Wilfred/difftastic)
A **structural (AST-aware) diff** tool rather than a line-based interactive reviewer. Difftastic excels at comparing refactored code where indentation or formatting changed dramatically; use it as your `git difftool` or pager when structural semantics matter more than interactive review workflows.

### [delta](https://github.com/dandavison/delta), [diff-so-fancy](https://github.com/so-fancy/diff-so-fancy), and GNU [diff](https://www.gnu.org/software/diffutils/)
Classic **stream / pager** tools designed to sit behind `git diff` and `git show` in your scrollback:
- **`delta`** adds rich syntax highlighting, side-by-side or unified output, merge-conflict formatting, and an optional `gitu`-backed `--tui` mode.
- **`diff-so-fancy`** focuses on clean, readable plain-text Git diffs without full syntax highlighting.
- **GNU `diff`** is the ubiquitous POSIX baseline for scripting and patch generation.
