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
        "value": "#4b5563",
        "value_strong": "#111827",
        "muted": "#6b7280",
        "axis": "#6b7280",
        "grid": "#e5e7eb",
        "divider": "#e5e7eb",
        "bars": {
            "lumiere": ("#b67b15", "1"),
            "plannotator-tui": ("#7c3aed", "0.72"),
            "hunk": ("#16a34a", "0.72"),
            "lumen": ("#94a3b8", "0.72"),
        },
    },
    "dark": {
        "bg": "#0d1117",
        "title": "#f0f6fc",
        "subtitle": "#8b949e",
        "label": "#e6edf3",
        "value": "#9ca3af",
        "value_strong": "#f0f6fc",
        "muted": "#8b949e",
        "axis": "#8b949e",
        "grid": "#21262d",
        "divider": "#30363d",
        "bars": {
            "lumiere": ("#f0b849", "1"),
            "plannotator-tui": ("#a78bfa", "0.75"),
            "hunk": ("#4ade80", "0.72"),
            "lumen": ("#6e7681", "0.75"),
        },
    },
}

SECTIONS = [
    {
        "title": "Startup Time",
        "subtitle": "launch to first diff on screen (lower is better)",
        "unit": "ms",
        "max_val": 900.0,
        "ticks": [0, 150, 300, 450, 600, 750, 900],
        "rows": [
            ("start, small repository", "Start (small, 3 files)"),
            ("start, many repository", "Start (many, 300 files)"),
            ("start, large repository", "Start (large, 360k lines)"),
            ("start, ignored repository", "Start (8k ignored dirs)"),
        ],
    },
    {
        "title": "Interactive UI Latency",
        "subtitle": "in-session response time (lower is better)",
        "unit": "ms",
        "max_val": 160.0,
        "ticks": [0, 20, 40, 60, 80, 100, 120, 140, 160],
        "rows": [
            ("scroll step, p95", "Scroll p95 (6k-line file)"),
            ("sidebar toggle, many", "Sidebar toggle (300 files)"),
            ("terminal resize", "Terminal resize"),
        ],
    },
    {
        "title": "Baseline Memory",
        "subtitle": "peak RSS on small repo (lower is better)",
        "unit": "MB",
        "max_val": 220.0,
        "ticks": [0, 50, 100, 150, 200],
        "rows": [
            ("peak memory, small repository", "Peak memory (small)"),
        ],
    },
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
    row_pitch = group_h + 18

    # Compute section vertical layout dynamically
    y = 96
    section_layouts = []
    for idx, sec in enumerate(SECTIONS):
        if idx > 0:
            divider_y = y + 6
            y = divider_y + 28
        else:
            divider_y = None
        title_y = y
        start_y = title_y + 18
        sec_h = len(sec["rows"]) * row_pitch - 12
        axis_y = start_y + sec_h + 6
        section_layouts.append((sec, divider_y, title_y, start_y, axis_y))
        y = axis_y + 28

    footer_y = y + 10
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
        "    .val-strong { font-size: 11px; font-weight: 700; font-variant-numeric: tabular-nums; }",
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
        fill, opacity = c["bars"][tool_id]
        parts.append(
            f'  <rect x="{lx}" y="{ly - 10}" width="11" height="11" rx="2" fill="{fill}" fill-opacity="{opacity}"/>'
        )
        parts.append(
            f'  <text x="{lx + 17}" y="{ly}" class="legend" fill="{c["subtitle"]}">{html.escape(display)}</text>'
        )
        lx += 24 + int(len(display) * 7.6) + 24

    for sec, divider_y, title_y, start_y, axis_y in section_layouts:
        if divider_y is not None:
            parts.append(
                f'  <line x1="28" y1="{divider_y}" x2="{width - 28}" y2="{divider_y}" '
                f'stroke="{c["divider"]}" stroke-width="1"/>'
            )

        parts.append(
            f'  <text x="28" y="{title_y}" class="section" fill="{c["title"]}">'
            f'{html.escape(sec["title"])}'
            f'<tspan dx="10" class="legend" font-weight="400" fill="{c["subtitle"]}">'
            f'· {html.escape(sec["subtitle"])}</tspan></text>'
        )

        max_val = sec["max_val"]
        unit = sec["unit"]
        for tick in sec["ticks"]:
            x = left + (tick / max_val) * plot_w
            parts.append(
                f'  <line x1="{x:.1f}" y1="{start_y - 4}" x2="{x:.1f}" y2="{axis_y}" '
                f'stroke="{c["grid"]}" stroke-width="1"/>'
            )
            tick_label = f"{tick} {unit}" if tick > 0 else "0"
            parts.append(
                f'  <text x="{x:.1f}" y="{axis_y + 15}" text-anchor="middle" class="axis" '
                f'fill="{c["axis"]}">{tick_label}</text>'
            )

        for r_idx, (metric, label) in enumerate(sec["rows"]):
            gy = start_y + r_idx * row_pitch
            vals = {tid: data[tid][metric] for tid, _ in SERIES}
            fastest = min(vals.values())
            lumiere_val = vals["lumiere"]
            parts.append(
                f'  <text x="{left - 14}" y="{gy + group_h / 2 + 4:.1f}" text-anchor="end" '
                f'class="label" fill="{c["label"]}">{html.escape(label)}</text>'
            )
            for s_idx, (tid, _) in enumerate(SERIES):
                v = vals[tid]
                by = gy + s_idx * (bar_h + bar_gap)
                bw = max(4.0, (v / max_val) * plot_w)
                fill, opacity = c["bars"][tid]
                is_best = v == fastest
                vcls = "val-strong" if is_best else "val"
                vcol = c["value_strong"] if is_best else c["value"]
                parts.append(
                    f'  <rect x="{left}" y="{by}" width="{bw:.1f}" height="{bar_h}" rx="2.5" '
                    f'fill="{fill}" fill-opacity="{opacity}"/>'
                )
                ratio_suffix = ""
                if tid != "lumiere" and lumiere_val > 0:
                    mult = v / lumiere_val
                    if mult >= 1.2:
                        ratio_suffix = f'<tspan dx="5" fill="{c["muted"]}" font-weight="400">({mult:.1f}×)</tspan>'
                parts.append(
                    f'  <text x="{left + bw + 7:.1f}" y="{by + 9.5:.1f}" class="{vcls}" '
                    f'fill="{vcol}">{fmt_raw(v, unit)}{ratio_suffix}</text>'
                )

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
