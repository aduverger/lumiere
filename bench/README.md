# Benchmark

Measures how fast Lumiere starts and reacts, so a change can be compared before and
after.

```bash
python3 bench/bench.py run before                      # the lumiere on PATH
python3 bench/bench.py run after --bin target/release/lumiere
python3 bench/bench.py compare before after
```

Linux only. Needs Python 3 and Git. A run takes about two minutes.

## How it measures

The script builds four Git repositories in a temporary folder, the same ones every
time. It runs the tool in a hidden terminal of 140 by 40 and reads the time until an
expected text is on screen. Each metric is measured five times (`--runs`); the tables
show the median.

| Repository | Content |
|---|---|
| `small` | 3 changed files |
| `many` | 300 changed files |
| `scroll` | 1 changed Python file of 6,000 lines (400 hunks, ~170 KB) |
| `large` | 1 small changed file and 1 changed file of about 5 MB |
| `ignored` | 3 changed files next to 8,000 folders that Git ignores |

## Metrics

| Metric | From | Until |
|---|---|---|
| start | launch | the first file is on screen |
| first key answered | a key pressed as soon as the first file is on screen | the second file is on screen |
| next file | the key press, one second later | the third file is on screen |
| sidebar toggle, many | toggle sidebar on 300-file repo | file tree hides and reopens (mean of both) |
| scroll step, median / p95 | half-page down on `scroll` (30 steps) | screen updates with scrolled lines |
| scroll output | | average PTY ANSI bytes written per scroll frame (KB) |
| terminal resize | `TIOCSWINSZ` + `SIGWINCH` (`140×40 ↔ 100×30`) | frame repaints at the new size (mean of both) |
| open the large file | the key press | the large file is on screen |
| large file, whole file view | `z` | no folded line is left on screen |
| file change to screen, watch | a file is written | its new text is on screen |
| idle processor use, watch | | share of one processor over 4 seconds |
| peak memory | | highest memory in use so far |

Lumiere gathers file changes for 500 ms before it reloads, so `file change to
screen` cannot be lower than that.

## Limits

- Files are in the disk cache: a first run after a restart is not measured.
- When run inside Rex, the start includes Lumiere's search for an agent, which
  depends on the panes that are open.
- Results are saved in `bench/results/`, which Git ignores: they compare runs on one
  machine, not machines.

## Another tool

Built-in `--tool` presets exist for `lumiere`, `plannotator-tui`, `hunk`, and `lumen`, and
`compare` accepts two or more saved labels:

```bash
python3 bench/bench.py run lumiere --tool lumiere --bin target/release/lumiere
python3 bench/bench.py run plannotator --tool plannotator-tui
python3 bench/bench.py run hunk --tool hunk
python3 bench/bench.py run lumen --tool lumen
python3 bench/bench.py compare lumiere plannotator hunk lumen
```

To benchmark another viewer, add an entry to `TOOLS` in `bench.py` with its command,
optional `--watch` argument, key bytes, and fold-toggle marker. To regenerate the
light/dark SVG benchmark charts from saved results:

```bash
python3 bench/chart.py --lumiere lumiere --hunk hunk --plannotator-tui plannotator --lumen lumen
```

## Measured results

Median of 5 runs on Linux (`aarch64`, 2 vCPU, 140×40 PTY, warm page cache) against official release binaries. Parentheses show each tool relative to Lumiere (lower is better):

| Metric | **Lumiere** (`0.1.0`) | **hunk** (`0.25.0`) | **plannotator-tui** (`0.12.0`, `diffs-rs`) | **lumen** (`2.32.0`) |
| --- | ---: | ---: | ---: | ---: |
| Start (`small` repo, 3 files) | **23.0 ms** | 497.6 ms *(21.6×)* | 52.1 ms *(2.3×)* | 578.3 ms *(25.1×)* |
| Peak memory (`small` repo) | **12.2 MB** | 201.8 MB *(16.5×)* | 15.1 MB *(1.2×)* | 37.3 MB *(3.1×)* |
| Start (`small` repo, `--watch`) | **19.4 ms** | 516.2 ms *(26.6×)* | *no watch* | 587.5 ms *(30.3×)* |
| File change to screen (`--watch`) | 508.3 ms³ | **285.1 ms** *(0.6×)* | *no watch* | 305.6 ms⁴ |
| Idle CPU (`--watch`, over 4 s) | **0.0%** | 0.3% | *no watch* | 1.3%⁴ |
| Start (`many` repo, 300 files) | **27.7 ms** | 849.7 ms *(30.7×)* | 240.6 ms *(8.7×)* | 659.1 ms *(23.8×)* |
| First key answered (`many`) | 10.4 ms | 69.9 ms *(6.7×)* | **1.6 ms** *(0.2×)* | 5.1 ms *(0.5×)* |
| Next file (`many`) | 4.6 ms | 12.4 ms *(2.7×)* | **1.0 ms** *(0.2×)* | 5.8 ms *(1.3×)* |
| Sidebar toggle (`many`, 300 files) | 1.6 ms | 153.9 ms *(96.2×)* | **1.0 ms** *(0.6×)* | 2.9 ms *(1.8×)* |
| Start (`many` repo, `--watch`) | **24.7 ms** | 935.2 ms *(37.9×)* | *no watch* | 665.0 ms *(26.9×)* |
| Start (`large` repo, 360k-line file) | **23.7 ms** | 573.3 ms *(24.2×)* | 167.5 ms *(7.1×)* | 759.4 ms *(32.0×)* |
| Open the 360k-line file (`5.3 MiB`) | **166.5 ms** | *skipped¹* | *streamed²* | 5,640.0 ms *(33.9×)* |
| Toggle whole-file view (`5.3 MiB`) | **110.2 ms** | *skipped¹* | *capped (5 MiB)²* | *n/a* |
| Peak memory (`large` repo) | 155.7 MB | 191.6 MB¹ *(1.2×)* | **15.3 MB²** *(0.1×)* | 1,105.6 MB *(7.1×)* |
| Start (`large` repo, `--watch`) | **20.3 ms** | 680.8 ms *(33.5×)* | *no watch* | 761.3 ms *(37.5×)* |
| Start (`ignored` repo, 8k `.venv` dirs) | **24.4 ms** | 469.2 ms *(19.2×)* | 48.3 ms *(2.0×)* | 593.1 ms *(24.3×)* |
| Start (`ignored` repo, `--watch`) | **22.4 ms** | 490.3 ms *(21.9×)* | *no watch* | 990.3 ms *(44.2×)* |
| Scroll step, median (6k-line file) | **0.7 ms** | 1.1 ms *(1.6×)* | **0.7 ms** *(1.0×)* | **0.7 ms** *(1.0×)* |
| Scroll step, p95 (6k-line file) | **0.9 ms** | 37.8 ms *(42.0×)* | 1.5 ms *(1.7×)* | 1.0 ms *(1.1×)* |
| Scroll output (PTY bytes/frame) | 4.2 KB | 4.1 KB *(1.0×)* | **2.2 KB** *(0.5×)* | 6.8 KB *(1.6×)* |
| Terminal resize (`140×40 ↔ 100×30`) | 3.1 ms | 103.8 ms *(33.5×)* | **1.1 ms** *(0.4×)* | 2.8 ms *(0.9×)* |

1. **`hunk`** prints `File too large to render automatically.` on the 5.3 MiB file.
2. **`plannotator-tui`** streams only patch hunks (`15.3 MB` RSS) and caps whole-file view at 5 MiB.
3. **Lumiere** intentionally gathers filesystem events for a 500 ms debounce window (`GATHERING_TIME`) before reloading.
4. **`lumen 2.32.0`** reloads continuously on `EventKind::Access` events in `--watch` mode.
