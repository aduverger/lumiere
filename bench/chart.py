#!/usr/bin/env python3
"""Generate Ghostty-style light and dark SVG benchmark charts from bench/results/*.json."""

import argparse
import html
import json
import statistics
from pathlib import Path

RESULTS = Path(__file__).resolve().parent / "results"
ROOT = Path(__file__).resolve().parent.parent
ASSET_DIRS = [ROOT / "assets", ROOT / "docs" / "src" / "assets"]

SERIES = [
    ("lumiere", "Lumiere (0.1.0)"),
    ("plannotator-tui", "plannotator-tui (0.12.0)"),
    ("hunk", "hunk (0.25.0)"),
    ("lumen", "lumen (2.32.0)"),
]

THEMES = {
    "light": {
        "bg": "#ffffff",
        "title": "#111827",
        "subtitle": "#4b5563",
        "label": "#1f2937",
        "value": "#374151",
        "value_strong": "#111827",
        "axis": "#6b7280",
        "grid": "#e5e7eb",
        "baseline": "#9ca3af",
        "divider": "#e5e7eb",
        "bars": {
            "lumiere": "#e09f1f",
            "plannotator-tui": "#7c3aed",
            "hunk": "#16a34a",
            "lumen": "#94a3b8",
        },
    },
    "dark": {
        "bg": "#0d1117",
        "title": "#f0f6fc",
        "subtitle": "#8b949e",
        "label": "#e6edf3",
        "value": "#c9d1d9",
        "value_strong": "#f0f6fc",
        "axis": "#8b949e",
        "grid": "#21262d",
        "baseline": "#6e7681",
        "divider": "#30363d",
        "bars": {
            "lumiere": "#ffc857",
            "plannotator-tui": "#a78bfa",
            "hunk": "#4ade80",
            "lumen": "#6e7681",
        },
    },
}

SPEEDUP_ROWS = [
    ("start, small repository", "Start (small, 3 files)", "ms"),
    ("start, many repository", "Start (many, 300 files)", "ms"),
    ("start, large repository", "Start (large, 360k lines)", "ms"),
    ("start, ignored repository", "Start (8k ignored dirs)", "ms"),
    ("peak memory, small repository", "Peak memory (small)", "MB"),
]

LATENCY_ROWS = [
    ("scroll step, p95", "Scroll p95 (6k-line file)", "ms"),
    ("sidebar toggle, many", "Sidebar toggle (300 files)", "ms"),
    ("terminal resize", "Terminal resize", "ms"),
]


def load_medians(labels_by_tool):
    data = {}
    for tool_id, label in labels_by_tool.items():
        raw = json.loads((RESULTS / f"{label}.json").read_text())
        data[tool_id] = {
            metric: statistics.median(values)
            for metric, values in raw["metrics"].items()
            if values
        }
    return data


def fmt_raw(value, unit):
    if value >= 100:
        return f"{value:.0f} {unit}"
    return f"{value:.1f} {unit}"


def render_svg(data, theme_name):
    c = THEMES[theme_name]
    width = 920
    left = 205
    right = 128
    plot_w = width - left - right

    bar_h = 12
    bar_gap = 4
    group_h = len(SERIES) * bar_h + (len(SERIES) - 1) * bar_gap
    row_pitch = group_h + 20

    top_header = 82
    sec1_title_y = top_header + 16
    sec1_start_y = sec1_title_y + 18
    sec1_height = len(SPEEDUP_ROWS) * row_pitch - 12
    sec1_axis_y = sec1_start_y + sec1_height + 6

    divider_y = sec1_axis_y + 34
    sec2_title_y = divider_y + 28
    sec2_start_y = sec2_title_y + 18
    sec2_height = len(LATENCY_ROWS) * row_pitch - 12
    sec2_axis_y = sec2_start_y + sec2_height + 6

    footer_y = sec2_axis_y + 36
    height = footer_y + 22

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}" role="img" aria-label="Lumiere Benchmark Comparison">',
        "  <style>",
        "    text { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif; }",
        "    .title { font-size: 22px; font-weight: 700; }",
        "    .section { font-size: 13px; font-weight: 600; }",
        "    .legend { font-size: 12px; }",
        "    .label { font-size: 12.5px; }",
        "    .val { font-size: 11px; font-variant-numeric: tabular-nums; }",
        "    .val-strong { font-size: 11px; font-weight: 600; font-variant-numeric: tabular-nums; }",
        "    .axis { font-size: 10.5px; font-variant-numeric: tabular-nums; }",
        "    .footer { font-size: 10.5px; }",
        "  </style>",
        f'  <rect width="{width}" height="{height}" rx="12" fill="{c["bg"]}"/>',
        f'  <text x="28" y="38" class="title" fill="{c["title"]}">Lumiere Benchmark Comparison</text>',
    ]

    # Legend
    lx = 28
    ly = 62
    for tool_id, display in SERIES:
        parts.append(
            f'  <rect x="{lx}" y="{ly - 10}" width="11" height="11" rx="2" fill="{c["bars"][tool_id]}"/>'
        )
        parts.append(
            f'  <text x="{lx + 17}" y="{ly}" class="legend" fill="{c["subtitle"]}">{html.escape(display)}</text>'
        )
        lx += 24 + int(len(display) * 7.6) + 24

    # Section 1: Startup & Memory (Speedup vs slowest, higher is faster)
    parts.append(
        f'  <text x="28" y="{sec1_title_y}" class="section" fill="{c["title"]}">'
        f'Startup &amp; Baseline Memory'
        f'<tspan dx="10" class="legend" font-weight="400" fill="{c["subtitle"]}">'
        f'· speedup / efficiency vs. slowest (higher is better)</tspan></text>'
    )

    speedup_max = 35.0
    ticks1 = [0, 5, 10, 15, 20, 25, 30, 35]
    for tick in ticks1:
        x = left + (tick / speedup_max) * plot_w
        parts.append(
            f'  <line x1="{x:.1f}" y1="{sec1_start_y - 4}" x2="{x:.1f}" y2="{sec1_axis_y}" '
            f'stroke="{c["grid"]}" stroke-width="1"/>'
        )
        parts.append(
            f'  <text x="{x:.1f}" y="{sec1_axis_y + 15}" text-anchor="middle" class="axis" '
            f'fill="{c["axis"]}">{tick}×</text>'
        )

    base1_x = left + (1.0 / speedup_max) * plot_w
    parts.append(
        f'  <line x1="{base1_x:.1f}" y1="{sec1_start_y - 4}" x2="{base1_x:.1f}" y2="{sec1_axis_y}" '
        f'stroke="{c["baseline"]}" stroke-width="1" stroke-dasharray="2.5,2.5"/>'
    )

    for r_idx, (metric, label, unit) in enumerate(SPEEDUP_ROWS):
        gy = sec1_start_y + r_idx * row_pitch
        vals = {tid: data[tid][metric] for tid, _ in SERIES}
        slowest = max(vals.values())
        fastest = min(vals.values())
        parts.append(
            f'  <text x="{left - 14}" y="{gy + group_h / 2 + 4:.1f}" text-anchor="end" '
            f'class="label" fill="{c["label"]}">{html.escape(label)}</text>'
        )
        for s_idx, (tid, _) in enumerate(SERIES):
            v = vals[tid]
            speedup = slowest / v
            by = gy + s_idx * (bar_h + bar_gap)
            bw = max(3.0, (speedup / speedup_max) * plot_w)
            is_best = v == fastest
            vcls = "val-strong" if is_best else "val"
            vcol = c["value_strong"] if is_best else c["value"]
            parts.append(
                f'  <rect x="{left}" y="{by}" width="{bw:.1f}" height="{bar_h}" rx="2.5" '
                f'fill="{c["bars"][tid]}"/>'
            )
            parts.append(
                f'  <text x="{left + bw + 7:.1f}" y="{by + 9.5:.1f}" class="{vcls}" '
                f'fill="{vcol}">{speedup:.1f}× ({fmt_raw(v, unit)})</text>'
            )

    # Divider
    parts.append(
        f'  <line x1="28" y1="{divider_y}" x2="{width - 28}" y2="{divider_y}" '
        f'stroke="{c["divider"]}" stroke-width="1"/>'
    )

    # Section 2: Interactive UI Latency (ms, lower is better)
    parts.append(
        f'  <text x="28" y="{sec2_title_y}" class="section" fill="{c["title"]}">'
        f'Interactive UI Latency'
        f'<tspan dx="10" class="legend" font-weight="400" fill="{c["subtitle"]}">'
        f'· wall-clock response time in ms (lower is better)</tspan></text>'
    )

    lat_max = 160.0
    ticks2 = [0, 20, 40, 60, 80, 100, 120, 140, 160]
    for tick in ticks2:
        x = left + (tick / lat_max) * plot_w
        parts.append(
            f'  <line x1="{x:.1f}" y1="{sec2_start_y - 4}" x2="{x:.1f}" y2="{sec2_axis_y}" '
            f'stroke="{c["grid"]}" stroke-width="1"/>'
        )
        parts.append(
            f'  <text x="{x:.1f}" y="{sec2_axis_y + 15}" text-anchor="middle" class="axis" '
            f'fill="{c["axis"]}">{tick} ms</text>'
        )

    for r_idx, (metric, label, unit) in enumerate(LATENCY_ROWS):
        gy = sec2_start_y + r_idx * row_pitch
        vals = {tid: data[tid][metric] for tid, _ in SERIES}
        fastest = min(vals.values())
        parts.append(
            f'  <text x="{left - 14}" y="{gy + group_h / 2 + 4:.1f}" text-anchor="end" '
            f'class="label" fill="{c["label"]}">{html.escape(label)}</text>'
        )
        for s_idx, (tid, _) in enumerate(SERIES):
            v = vals[tid]
            by = gy + s_idx * (bar_h + bar_gap)
            bw = max(3.0, (v / lat_max) * plot_w)
            is_best = v == fastest
            vcls = "val-strong" if is_best else "val"
            vcol = c["value_strong"] if is_best else c["value"]
            parts.append(
                f'  <rect x="{left}" y="{by}" width="{bw:.1f}" height="{bar_h}" rx="2.5" '
                f'fill="{c["bars"][tid]}"/>'
            )
            parts.append(
                f'  <text x="{left + bw + 7:.1f}" y="{by + 9.5:.1f}" class="{vcls}" '
                f'fill="{vcol}">{fmt_raw(v, unit)}</text>'
            )

    # Footer
    parts.append(
        f'  <text x="28" y="{footer_y}" class="footer" fill="{c["axis"]}">'
        f'140×40 PTY · median of 5 runs · Linux aarch64 (2 vCPU) · Data from bench/bench.py</text>'
    )
    parts.append("</svg>\n")
    return "\n".join(parts)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lumiere", default="ui-lumiere")
    parser.add_argument("--plannotator-tui", dest="plannotator_tui", default="ui-plannotator-tui")
    parser.add_argument("--hunk", default="ui-hunk")
    parser.add_argument("--lumen", default="ui-lumen")
    args = parser.parse_args()

    data = load_medians(
        {
            "lumiere": args.lumiere,
            "plannotator-tui": args.plannotator_tui,
            "hunk": args.hunk,
            "lumen": args.lumen,
        }
    )
    for theme in ("light", "dark"):
        svg = render_svg(data, theme)
        for asset_dir in ASSET_DIRS:
            asset_dir.mkdir(parents=True, exist_ok=True)
            out = asset_dir / f"benchmarks-{theme}.svg"
            out.write_text(svg)
            print(f"wrote {out}")


if __name__ == "__main__":
    main()
