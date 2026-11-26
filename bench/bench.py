#!/usr/bin/env python3
"""Measures how fast a terminal diff viewer starts and reacts. See README.md."""

import argparse
import codecs
import fcntl
import json
import os
import pty
import re
import select
import shutil
import signal
import statistics
import struct
import subprocess
import sys
import tempfile
import termios
import time
from pathlib import Path

ROWS, COLUMNS = 40, 140
RESIZE_ROWS, RESIZE_COLUMNS = 30, 100
SCROLL_STEPS = 30
WAIT_LIMIT = 60.0
# Long enough for a tool to finish what it does after its first screen.
SETTLING_TIME = 1.0
# A control sequence, a title-like sequence, a character set choice, or any other escape.
ESCAPE_SEQUENCE = re.compile(
    r"\x1b(?:\[([?>=<]?)([0-9;:]*)([ -/]*[@-~])|\]([^\x07\x1b]*)(?:\x07|\x1b\\)|[()][0-9A-B]|[^\[\]()])"
)
RESULTS = Path(__file__).resolve().parent / "results"

# What the harness needs to know about a tool. `keys` are the bytes a terminal sends.
TOOLS = {
    "lumiere": {
        "command": ["lumiere", "diff"],
        "watch": ["--watch"],
        "keys": {
            "next_file": b"\x1bk",
            "toggle_view": b"z",
            "page_down": b"\x04",
            "toggle_sidebar": b"\t",
            "quit": b"q",
        },
        "sidebar_open_at_start": True,
        # On screen in the folded view only.
        "folded_text": "unchanged lines",
    },
    "lumen": {
        "command": ["lumen", "diff"],
        "watch": ["--watch"],
        "keys": {
            "next_file": b"\x0a",
            "page_down": b"\x04",
            "toggle_sidebar": b"\t",
            "quit": b"q",
        },
        "sidebar_open_at_start": True,
    },
    "hunk": {
        "command": ["hunk", "diff"],
        "watch": ["--watch"],
        "keys": {
            "next_file": b".",
            "toggle_view": b"z",
            "page_down": b"d",
            "toggle_sidebar": b"s",
            "quit": b"q",
        },
        "sidebar_open_at_start": False,
        # Hunk skips auto-rendering the 5.3 MB fixture ("File too large to render automatically.").
        "large_file_rendered": False,
        "unfolded_text": "Hide ",
    },
    "plannotator-tui": {
        "command": ["plannotator-tui", "review"],
        "watch": None,
        "keys": {
            "next_file": b"n",
            "toggle_view": b"o",
            "page_down": b"\x04",
            "focus_sidebar": b"f",
            "toggle_sidebar": b"f",
            "quit": b"q",
        },
        "sidebar_open_at_start": True,
        # Continuous stream already shows z_large.py's first hunk at start, and whole-file view caps at 5 MiB.
        "large_file_rendered": False,
        "folded_text": "Context not included in patch",
    },
}


# --- test repositories -------------------------------------------------------------


def git(repository, *arguments):
    subprocess.run(["git", "-C", str(repository), *arguments], check=True, capture_output=True)


def source(lines, marker):
    """A Python file of `lines` lines; `marker` is on its second line."""
    body = [f'NAME = "{marker}"']
    body += [f"def value_{n}(x):\n    return x * {n} + 1\n" for n in range((lines - 2) // 3)]
    return "# generated for the benchmark\n" + "\n".join(body) + "\n"


def changed(text, marker, new_marker, every):
    """`text` with another marker and one line added every `every` lines."""
    lines = text.replace(marker, new_marker).split("\n")
    for index in range(len(lines) - 1, 10, -every):
        lines.insert(index, f"# added near line {index}")
    return "\n".join(lines)


def build_repository(root, files, ignored_folders=0):
    """`files` maps a path to (line count, lines between two additions)."""
    root.mkdir(parents=True)
    git(root, "init", "-q", "-b", "main")
    git(root, "config", "user.email", "bench@example.com")
    git(root, "config", "user.name", "Bench")
    originals = {}
    for path, (lines, _) in files.items():
        originals[path] = source(lines, f"before_{Path(path).stem}")
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_text(originals[path])
    if ignored_folders:
        (root / ".gitignore").write_text(".venv/\n")
    git(root, "add", ".")
    git(root, "commit", "-q", "-m", "base")
    for path, (_, every) in files.items():
        stem = Path(path).stem
        (root / path).write_text(changed(originals[path], f"before_{stem}", f"after_{stem}", every))
    for n in range(ignored_folders):
        folder = root / ".venv" / f"lib{n // 100:02}" / f"package{n % 100:02}"
        folder.mkdir(parents=True)
        for module in range(6):
            (folder / f"module{module}.py").write_text("x = 1\n")


def build_repositories(root):
    build_repository(root / "small", {f"src/file{n}.py": (120, 40) for n in range(3)})
    build_repository(root / "many", {f"src/dir{n // 20:02}/file{n:03}.py": (240, 60) for n in range(300)})
    build_repository(root / "scroll", {"src/scroll.py": (6_000, 15)})
    build_repository(root / "large", {"a_small.py": (120, 40), "z_large.py": (360_000, 6_000)})
    build_repository(root / "ignored", {f"src/file{n}.py": (120, 40) for n in range(3)}, ignored_folders=8000)


# --- a terminal, as far as a full-screen program needs one -------------------------


class Screen:
    """Keeps the characters a program puts on screen and answers its questions."""

    def __init__(self, rows=ROWS, columns=COLUMNS):
        self.rows = rows
        self.columns = columns
        self.cells = [[" "] * columns for _ in range(rows)]
        self.row = self.column = 0
        self.pending = ""

    def text(self):
        return "\n".join("".join(row) for row in self.cells)

    def feed(self, text):
        """Applies program output; returns what the terminal answers."""
        answers = b""
        data = self.pending + text
        position = 0
        while position < len(data):
            character = data[position]
            if character != "\x1b":
                self.put(character)
                position += 1
                continue
            match = ESCAPE_SEQUENCE.match(data, position)
            if not match:
                break  # an escape sequence cut in two: wait for the rest
            if match.group(3):
                answers += self.control(match.group(1), match.group(2), match.group(3))
            elif match.group(4) == "11;?":
                answers += b"\x1b]11;rgb:1616/1818/1d1d\x07"
            position = match.end()
        self.pending = data[position:]
        return answers

    def put(self, character):
        if character == "\r":
            self.column = 0
        elif character == "\n":
            self.row = min(self.row + 1, self.rows - 1)
        elif character == "\b":
            self.column = max(self.column - 1, 0)
        elif character >= " ":
            if self.column >= self.columns:
                self.column = 0
                self.row = min(self.row + 1, self.rows - 1)
            self.cells[self.row][self.column] = character
            self.column += 1

    def control(self, prefix, parameters, final):
        numbers = [int(n) if n else 0 for n in re.split("[;:]", parameters)]
        first = numbers[0]
        if prefix == "?" and final in "hl" and first in (47, 1047, 1049):
            self.cells = [[" "] * self.columns for _ in range(self.rows)]
        elif prefix:
            pass
        elif final in "Hf":
            self.row = min(max(first, 1), self.rows) - 1
            self.column = min(max(numbers[1] if len(numbers) > 1 else 1, 1), self.columns) - 1
        elif final == "A":
            self.row = max(self.row - max(first, 1), 0)
        elif final == "B":
            self.row = min(self.row + max(first, 1), self.rows - 1)
        elif final == "C":
            self.column = min(self.column + max(first, 1), self.columns - 1)
        elif final == "D":
            self.column = max(self.column - max(first, 1), 0)
        elif final == "G":
            self.column = min(max(first, 1), self.columns) - 1
        elif final == "d":
            self.row = min(max(first, 1), self.rows) - 1
        elif final == "K":
            start, end = {0: (self.column, self.columns), 1: (0, self.column + 1)}.get(
                first, (0, self.columns)
            )
            self.cells[self.row][start:end] = [" "] * (end - start)
        elif final == "J" and first in (2, 3):
            self.cells = [[" "] * self.columns for _ in range(self.rows)]
        elif final == "J" and first == 0:
            self.cells[self.row][self.column :] = [" "] * (self.columns - self.column)
            for row in range(self.row + 1, self.rows):
                self.cells[row] = [" "] * self.columns
        elif final == "n" and first == 5:
            return b"\x1b[0n"
        elif final == "n" and first == 6:
            return f"\x1b[{self.row + 1};{self.column + 1}R".encode()
        elif final == "c":
            return b"\x1b[?62;c"
        return b""


class Session:
    """One run of a tool inside a terminal of ROWS x COLUMNS."""

    def __init__(self, command, folder, state_folder):
        self.screen = Screen()
        self.bytes_read = 0
        # Keeps the bytes of a character cut in two for the next read.
        self.decoder = codecs.getincrementaldecoder("utf-8")(errors="ignore")
        environment = {**os.environ, "TERM": "xterm-256color", "COLORTERM": "truecolor"}
        environment["XDG_STATE_HOME"] = str(state_folder)
        self.started = time.monotonic()
        self.pid, self.terminal = pty.fork()
        if self.pid == 0:
            os.chdir(folder)
            os.execvpe(command[0], command, environment)
        fcntl.ioctl(self.terminal, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLUMNS, 0, 0))

    def read(self, timeout):
        readable, _, _ = select.select([self.terminal], [], [], timeout)
        if not readable:
            return
        try:
            data = os.read(self.terminal, 1 << 16)
        except OSError:
            return
        self.bytes_read += len(data)
        answers = self.screen.feed(self.decoder.decode(data))
        if answers:
            os.write(self.terminal, answers)

    def resize(self, rows, columns):
        self.screen = Screen(rows, columns)
        started = time.monotonic()
        fcntl.ioctl(self.terminal, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0))
        os.kill(self.pid, signal.SIGWINCH)
        return started

    def seconds_until(self, on_screen, since=None):
        """Seconds from `since` (default: launch) until `on_screen(text)` holds."""
        since = self.started if since is None else since
        while time.monotonic() - since < WAIT_LIMIT:
            if on_screen(self.screen.text()):
                return time.monotonic() - since
            self.read(0.02)
        raise TimeoutError(self.screen.text())

    def press(self, keys):
        os.write(self.terminal, keys)
        return time.monotonic()

    def idle(self, seconds):
        until = time.monotonic() + seconds
        while time.monotonic() < until:
            self.read(0.05)

    def cpu_seconds(self):
        """Processor time used so far, finished child processes included."""
        fields = Path(f"/proc/{self.pid}/stat").read_text().rsplit(")", 1)[1].split()
        return sum(int(ticks) for ticks in fields[11:15]) / os.sysconf("SC_CLK_TCK")

    def peak_memory_mb(self):
        status = Path(f"/proc/{self.pid}/status").read_text()
        return int(re.search(r"VmHWM:\s+(\d+) kB", status).group(1)) / 1024

    def close(self, quit_key):
        try:
            os.write(self.terminal, quit_key)
        except OSError:
            pass
        until = time.monotonic() + 2
        while time.monotonic() < until:
            if os.waitpid(self.pid, os.WNOHANG)[0]:
                break
            self.read(0.02)
        else:
            os.kill(self.pid, signal.SIGKILL)
            os.waitpid(self.pid, 0)
        os.close(self.terminal)


# --- what is measured --------------------------------------------------------------


def shows(text):
    return lambda screen: text in screen


def measure_once(tool, repositories, state_folder):
    """One pass over every metric: {metric name: value}."""
    keys = tool["keys"]
    watch_modes = (False, True) if tool.get("watch") else (False,)
    values = {}
    first_file = {"small": "after_file0", "many": "after_file000", "large": "after_a_small", "ignored": "after_file0"}
    for name, marker in first_file.items():
        for watch in watch_modes:
            command = tool["command"] + (tool["watch"] if watch else [])
            run = Session(command, repositories / name, state_folder)
            try:
                label = f"start, {name} repository" + (", watch" if watch else "")
                values[label] = run.seconds_until(shows(marker)) * 1000
                if name == "small" and not watch:
                    run.idle(SETTLING_TIME)
                    values["peak memory, small repository"] = run.peak_memory_mb()
                if name == "many" and not watch:
                    # Pressed at once: includes what the tool still does after its first screen.
                    pressed = run.press(keys["next_file"])
                    values["first key answered"] = run.seconds_until(shows("after_file001"), pressed) * 1000
                    run.idle(SETTLING_TIME)
                    pressed = run.press(keys["next_file"])
                    values["next file"] = run.seconds_until(shows("after_file002"), pressed) * 1000
                    if "toggle_sidebar" in keys:
                        run.idle(0.3)
                        if "focus_sidebar" in keys:
                            run.press(keys["focus_sidebar"])
                            run.idle(0.15)
                        open_now = tool.get("sidebar_open_at_start", True)
                        t1_wait = (lambda scr: "file001" not in scr) if open_now else shows("file001")
                        t2_wait = shows("file001") if open_now else (lambda scr: "file001" not in scr)
                        pressed = run.press(keys["toggle_sidebar"])
                        t1 = run.seconds_until(t1_wait, pressed) * 1000
                        run.idle(0.2)
                        pressed = run.press(keys["toggle_sidebar"])
                        t2 = run.seconds_until(t2_wait, pressed) * 1000
                        values["sidebar toggle, many"] = (t1 + t2) / 2
                if name == "large" and not watch:
                    run.idle(SETTLING_TIME)
                    if tool.get("large_file_rendered", True):
                        pressed = run.press(keys["next_file"])
                        values["open the large file"] = run.seconds_until(shows("after_z_large"), pressed) * 1000
                        if "toggle_view" in keys:
                            pressed = run.press(keys["toggle_view"])
                            if "folded_text" in tool:
                                folded = tool["folded_text"]
                                unfolded = lambda screen: folded not in screen
                            else:
                                unfolded = shows(tool["unfolded_text"])
                            values["large file, whole file view"] = run.seconds_until(unfolded, pressed) * 1000
                    values["peak memory, large file"] = run.peak_memory_mb()
                if name == "small" and watch:
                    run.idle(1.5)
                    file = repositories / "small" / "src/file0.py"
                    original = file.read_text()
                    file.write_text(original.replace("after_file0", "edited_file0"))
                    changed_at = time.monotonic()
                    values["file change to screen, watch"] = (
                        run.seconds_until(shows("edited_file0"), changed_at) * 1000
                    )
                    file.write_text(original)
                    run.idle(1.5)
                    used = run.cpu_seconds()
                    run.idle(4)
                    values["idle processor use, watch"] = (run.cpu_seconds() - used) / 4 * 100
            finally:
                run.close(keys["quit"])

    if "page_down" in keys:
        run = Session(tool["command"], repositories / "scroll", state_folder)
        try:
            run.seconds_until(shows("after_scroll"))
            run.idle(SETTLING_TIME)
            latencies = []
            previous = run.screen.text()
            bytes_before = run.bytes_read
            for _ in range(SCROLL_STEPS):
                pressed = run.press(keys["page_down"])
                elapsed = run.seconds_until(lambda scr, prev=previous: scr != prev, pressed) * 1000
                latencies.append(elapsed)
                previous = run.screen.text()
            run.idle(0.1)
            ordered = sorted(latencies)
            p95_index = min(len(ordered) - 1, int(0.95 * (len(ordered) - 1)))
            values["scroll step, median"] = statistics.median(ordered)
            values["scroll step, p95"] = ordered[p95_index]
            values["scroll output"] = (run.bytes_read - bytes_before) / SCROLL_STEPS / 1024

            empty_small = Screen(RESIZE_ROWS, RESIZE_COLUMNS).text()
            started = run.resize(RESIZE_ROWS, RESIZE_COLUMNS)
            r1 = run.seconds_until(lambda scr: scr != empty_small, started) * 1000
            run.idle(0.15)
            empty_full = Screen(ROWS, COLUMNS).text()
            started = run.resize(ROWS, COLUMNS)
            r2 = run.seconds_until(lambda scr: scr != empty_full, started) * 1000
            values["terminal resize"] = (r1 + r2) / 2
        finally:
            run.close(keys["quit"])
    return values


UNITS = {"peak memory": "MB", "idle processor": "%", "scroll output": "KB"}


def unit_of(metric):
    return next((unit for start, unit in UNITS.items() if metric.startswith(start)), "ms")


def run(arguments):
    tool = dict(TOOLS[arguments.tool])
    if arguments.bin:
        tool["command"] = [str(Path(arguments.bin).resolve())] + tool["command"][1:]
    work = Path(tempfile.mkdtemp(prefix="lumiere-bench-"))
    try:
        print("building the test repositories...", file=sys.stderr)
        build_repositories(work / "repositories")
        metrics = {}
        for number in range(arguments.runs):
            print(f"run {number + 1} of {arguments.runs}", file=sys.stderr)
            for metric, value in measure_once(tool, work / "repositories", work / "state").items():
                metrics.setdefault(metric, []).append(round(value, 1))
    finally:
        shutil.rmtree(work, ignore_errors=True)
    result = {
        "label": arguments.label,
        "tool": arguments.tool,
        "command": tool["command"],
        "date": time.strftime("%Y-%m-%d %H:%M"),
        "processors": os.cpu_count(),
        "metrics": metrics,
    }
    RESULTS.mkdir(exist_ok=True)
    (RESULTS / f"{arguments.label}.json").write_text(json.dumps(result, indent=2) + "\n")
    print(f"{'metric':<34} {'median':>10} {'min':>10} {'max':>10}")
    for metric, values in metrics.items():
        unit = unit_of(metric)
        print(
            f"{metric:<34} {statistics.median(values):>7.1f} {unit:<2} {min(values):>7.1f} {unit:<2} {max(values):>7.1f} {unit:<2}"
        )


def compare(arguments):
    labels = arguments.labels
    loaded = [json.loads((RESULTS / f"{label}.json").read_text()) for label in labels]
    if len(labels) == 2:
        before, after = loaded
        print(f"{'metric':<34} {labels[0]:>12} {labels[1]:>12} {'change':>9}")
        for metric, values in before["metrics"].items():
            if metric not in after["metrics"]:
                continue
            old, new = statistics.median(values), statistics.median(after["metrics"][metric])
            change = f"{(new - old) / old * 100:+.0f}%" if old else "n/a"
            unit = unit_of(metric)
            print(f"{metric:<34} {old:>9.0f} {unit:<2} {new:>9.0f} {unit:<2} {change:>9}")
        return

    all_metrics = list(dict.fromkeys(m for r in loaded for m in r["metrics"]))
    header = f"{'metric':<34}" + "".join(f" {label:>15}" for label in labels)
    print(header)
    for metric in all_metrics:
        unit = unit_of(metric)
        row = f"{metric:<34}"
        for r in loaded:
            values = r["metrics"].get(metric)
            if values is None:
                row += f" {'—':>15}"
            else:
                row += f" {statistics.median(values):>12.1f} {unit:<2}"
        print(row)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    run_parser = commands.add_parser("run", help="measure a tool and save the result")
    run_parser.add_argument("label", help="name of the result, e.g. before")
    run_parser.add_argument("--tool", default="lumiere", choices=sorted(TOOLS))
    run_parser.add_argument("--bin", help="program to run instead of the one on PATH")
    run_parser.add_argument("--runs", type=int, default=5)
    run_parser.set_defaults(action=run)
    compare_parser = commands.add_parser("compare", help="compare two or more saved results")
    compare_parser.add_argument("labels", nargs="+")
    compare_parser.set_defaults(action=compare)
    arguments = parser.parse_args()
    arguments.action(arguments)


if __name__ == "__main__":
    main()
