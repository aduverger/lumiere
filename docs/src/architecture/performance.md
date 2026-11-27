# Performance & Benchmarks

Lumiere is engineered to feel instantaneous on everyday commits while staying responsive on 300-file branches, 360,000-line generated files, and repositories with thousands of gitignored directories (such as `.venv/` or `node_modules/`).

## Benchmark suite

The repository includes an end-to-end PTY benchmark harness in [`bench/bench.py`](https://github.com/aduverger/lumiere/blob/main/bench/bench.py). It launches the compiled release binary inside a 140×40 pseudo-terminal, feeds real terminal escape replies and keystrokes, and measures wall-clock latency, peak RSS memory, and idle CPU usage across five synthetic Git repositories:

| Repository fixture | Contents |
|---|---|
| **`small`** | 3 modified Python files (120 lines each) |
| **`many`** | 300 modified Python files across 15 directories (240 lines each) |
| **`scroll`** | 1 modified Python file (6,000 lines, 400 hunks) |
| **`large`** | 1 small Python file + 1 modified 360,000-line Python file |
| **`ignored`** | `small` plus 8,000 gitignored `.venv/` subdirectories (48,000 files) |

### Running benchmarks locally

```bash
cargo build --release
python3 bench/bench.py run baseline --bin target/release/lumiere --runs 5
# ... make changes and rebuild ...
python3 bench/bench.py run candidate --bin target/release/lumiere --runs 5
python3 bench/bench.py compare baseline candidate
```

## Measured results

> [!TIP]
> For side-by-side benchmark numbers comparing **Lumiere** against **plannotator-tui**, **hunk**, and **lumen**, see [Comparison — Performance comparison](../comparison.md#performance-comparison).

Median of 5 runs on Linux (`aarch64`, 2 vCPU, 140×40 PTY, warm page cache) for `lumiere 0.1.0`:

| Metric | Median |
|---|---:|
| Start (`small` repo, 3 files) | **23.0 ms** |
| Peak memory (`small` repo) | **12.2 MB** |
| Start (`small` repo, `--watch`) | **19.4 ms** |
| File change to screen (`--watch`) | 508.3 ms |
| Idle CPU (`--watch`, over 4 s) | **0.0%** |
| Start (`many` repo, 300 files) | **27.7 ms** |
| First key answered (`many`) | **10.4 ms** |
| Next file (`many`) | **4.6 ms** |
| Sidebar toggle (`many`, 300 files) | **1.6 ms** |
| Start (`many` repo, `--watch`) | **24.7 ms** |
| Start (`large` repo, 360k lines) | **23.7 ms** |
| Open the 360k-line file (`5.3 MiB`) | **166.5 ms** |
| Toggle full-file view (`z`, `5.3 MiB`) | **110.2 ms** |
| Peak memory (`large` repo) | 155.7 MB |
| Start (`large` repo, `--watch`) | **20.3 ms** |
| Start (`ignored` repo, 8k `.venv` dirs) | **24.4 ms** |
| Start (`ignored` repo, `--watch`) | **22.4 ms** |
| Scroll step, median / p95 (6k-line file) | **0.7 ms** / **0.9 ms** |
| Terminal resize (`140×40 ↔ 100×30`) | **3.1 ms** |

> [!NOTE]
> `file change to screen, watch` is bounded below by the 500 ms `GATHERING_TIME` debounce window in `diff::watcher` so atomic editor save-and-rename sequences settle into a single reload.

## Key architectural optimizations

### 1. Non-blocking background `AgentWorker`

`review::agent::AgentWorker` runs all host queries (`Attach`, `Refresh`, `FindCandidates`) on a dedicated background thread over `std::sync::mpsc` channels so multiplexer CLI calls never stall the UI event loop.

### 2. Event-driven `dirty` redraws

`App` tracks a `dirty: bool` flag and only calls `terminal.draw(...)` when input arrives, the terminal resizes, a watcher event fires, `threads.json` changes on disk, an `AgentResponse` updates visible state, or a status notice expires. While idle in `--watch` mode, Lumiere consumes **0.0% CPU**.

### 3. Lazy per-language Tree-sitter compilation

Both `diff::highlight::config` and `diff::context` compile Tree-sitter `HighlightConfiguration` and scope `Query` objects lazily per language via `OnceLock`, avoiding upfront compilation of unused grammars.

### 4. Cached `ComputedDiff` and branch name

`ui::state::AppState` caches the active file's `ComputedDiff` (paired side-by-side rows, hunk indices, `LineStats`, and `max_old_width` / `max_new_width`) and the repository's current branch name. Per-frame rendering, footer statistics, and horizontal scroll clamping are O(1) instead of rescanning lines or invoking VCS lookups on every draw.

### 5. Lazy per-file blob loading, background stats, and batched reads

On Git repositories, startup reads only changed file paths and statuses (`A`/`M`/`D`) from tree deltas / `libgit2` status, then loads blob contents on demand as files are opened (or on `Ctrl+F` global search and for files with persisted threads). Repo-wide `+added -removed` totals are computed asynchronously on a background thread using `libgit2`'s C `Diff::stats()` (guarded by a generation counter against stale reloads). When all files must be loaded together, `VcsBackend::get_files_content_at_ref` resolves the revision tree once per batch.

### 6. Large-file threshold (>1 MB) and zero-alloc hot paths

- Files exceeding 1 MB skip full-file Tree-sitter parsing and scope extraction (`AppState::current_file_is_too_large_to_parse`), falling back to fast single-line highlighting of visible rows only while showing a `large file: colours are approximate` notice.
- `diff::context::extract_line` scans bytes directly to the target line in O(line length) instead of collecting every line of a file into a `Vec<&str>`.
- `ui::views::sidebar` checks directory path prefixes without heap-allocating `format!("{dir}/")` strings on every render pass.
- `ui::input` coalesces consecutive buffered mouse wheel events before scrolling and redrawing once.

### 7. Gitignore-aware watch startup on Linux

On Linux, `diff::watcher` installs non-recursive inotify watches only on non-gitignored directories (plus `.git/` metadata for commits/checkouts) instead of walking `.venv/` or `node_modules/`.

### 8. Release build profile

`Cargo.toml` configures `[profile.release]` with `lto = true`, `codegen-units = 1`, and `strip = "symbols"`, and omits unused `git2` network features (`https` / `vendored-openssl`, since remote operations go through `gh`).
