#!/usr/bin/env python3
"""Render four Codex visualization HTML fragments and PNG charts from metrics."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess


METRICS = (
    {
        "filename": "arrival-fee-by-turn-bin.html",
        "png_filename": "arrival-fee-by-turn-bin.png",
        "png_title": "Arrival order x Departure earnings",
        "root": "arrival-fee-by-turn-bin",
        "title": "入場順 × 退出時獲得額",
        "axis": "平均獲得額（円／グループ）",
        "key": "mean_fee",
        "series": 1,
        "format": "fee",
        "zero": True,
    },
    {
        "filename": "arrival-compactness-by-turn-bin.html",
        "png_filename": "arrival-compactness-by-turn-bin.png",
        "png_title": "Arrival order x Entry compactness",
        "root": "arrival-compactness-by-turn-bin",
        "title": "入場順 × 入場時コンパクト度",
        "axis": "平均コンパクト度",
        "key": "mean_entry_compactness",
        "series": 1,
        "format": "compactness",
        "zero": False,
    },
    {
        "filename": "arrival-rejection-rate-by-turn-bin.html",
        "png_filename": "arrival-rejection-rate-by-turn-bin.png",
        "png_title": "Arrival order x Rejection rate",
        "root": "arrival-rejection-rate-by-turn-bin",
        "title": "入場順 × 拒否率",
        "axis": "拒否率",
        "key": "rejection_rate",
        "series": 2,
        "format": "percent",
        "zero": True,
    },
    {
        "filename": "arrival-vacancy-rate-by-turn-bin.html",
        "png_filename": "arrival-vacancy-rate-by-turn-bin.png",
        "png_title": "Arrival order x Pre-arrival vacancy rate",
        "root": "arrival-vacancy-rate-by-turn-bin",
        "title": "入場順 × 入場直前の空きマス率",
        "axis": "平均空きマス率",
        "key": "mean_pre_arrival_vacancy_rate",
        "series": 3,
        "format": "percent",
        "zero": True,
    },
)

PNG_FONT = "Arial"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("metrics", type=Path)
    parser.add_argument("--output-dir", type=Path, required=True)
    return parser.parse_args()


def detail(metric_key: str, row: dict) -> str:
    label = f"{row['start']}–{row['end']} 番目"
    if metric_key == "mean_fee":
        return (
            f"{label}<br>平均: <b>{row['mean_fee']:,.0f} 円</b>"
            f"<br>合計: {row['total_fee']:,} 円"
            f"<br>入場させた数: {row['admitted_count']:,} / {row['group_count']:,}"
        )
    if metric_key == "mean_entry_compactness":
        return (
            f"{label}<br>平均: <b>{row['mean_entry_compactness']:.4f}</b>"
            f"<br>入場させた数: {row['admitted_count']:,} / {row['group_count']:,}"
        )
    if metric_key in ("rejection_rate", "mean_pre_arrival_vacancy_rate"):
        value_label = "拒否率" if metric_key == "rejection_rate" else "平均空きマス率"
        suffix = (
            f"<br>拒否: {row['rejected_count']:,} / {row['group_count']:,}"
            if metric_key == "rejection_rate"
            else ""
        )
        return f"{label}<br>{value_label}: <b>{row[metric_key] * 100:.2f}%</b>{suffix}"
    raise ValueError(metric_key)


def render_fragment(metrics: dict, config: dict) -> str:
    points = [
        {
            "start": row["start"],
            "end": row["end"],
            "value": row[config["key"]],
            "detail": detail(config["key"], row),
        }
        for row in metrics["bins"]
    ]
    if any(point["value"] is None for point in points):
        raise ValueError(f"{config['key']} is undefined for a bin")
    data_json = json.dumps(points, ensure_ascii=False, separators=(",", ":"))
    tick_format = "d3.format('.0%')" if config["format"] == "percent" else (
        "d => `${Math.round(d / 1000)}k`" if config["format"] == "fee" else "d3.format('.3f')"
    )
    return f'''<div id="{config['root']}">
  <style>
    #{config['root']} {{ color: var(--foreground); font-family: var(--font-sans, system-ui, sans-serif); }}
    #{config['root']} h3 {{ margin: 0 0 8px; font-weight: 500; }}
    #{config['root']} .plot {{ position: relative; }}
    #{config['root']} svg {{ display: block; width: 100%; height: auto; }}
    #{config['root']} text {{ fill: var(--foreground); font-size: 12px; }}
    #{config['root']} .tooltip {{ position: absolute; display: none; pointer-events: none; z-index: 2; padding: 7px 9px; background: var(--popover); color: var(--popover-foreground); border: 1px solid var(--border); font-size: 12px; white-space: nowrap; }}
  </style>
  <h3>{config['title']}（{metrics['bin_size']}グループ区切り）</h3>
  <div class="plot"><svg aria-label="{config['title']}の棒グラフ"></svg><div class="tooltip" role="tooltip"></div></div>
  <script src="https://cdn.jsdelivr.net/npm/d3@7.9.0/dist/d3.min.js"></script>
  <script>
  (() => {{
    const root=document.getElementById('{config['root']}'), data={data_json};
    const plot=root.querySelector('.plot'), svg=d3.select(root).select('svg'), tooltip=root.querySelector('.tooltip');
    function draw() {{
      const width=Math.max(360,plot.clientWidth||720),height=360,m={{top:12,right:18,bottom:62,left:76}},iw=width-m.left-m.right,ih=height-m.top-m.bottom;
      svg.attr('viewBox',`0 0 ${{width}} ${{height}}`).selectAll('*').remove();
      const x=d3.scaleBand().domain(data.map(d=>d.start)).range([m.left,width-m.right]).paddingInner(.18).paddingOuter(.06);
      const extent=d3.extent(data,d=>d.value),span=Math.max(extent[1]-extent[0],Math.abs(extent[1])*.05,1e-9),domain={str(config['zero']).lower()}?[0,extent[1]+span*.12]:[extent[0]-span*.14,extent[1]+span*.14];
      const y=d3.scaleLinear().domain(domain).nice().range([height-m.bottom,m.top]);
      svg.append('rect').attr('data-chart-frame','').attr('x',m.left).attr('y',m.top).attr('width',iw).attr('height',ih).attr('fill','none').attr('stroke','var(--border)');
      svg.append('g').attr('transform',`translate(${{m.left}},0)`).call(d3.axisLeft(y).ticks(5).tickFormat({tick_format})).call(g=>g.select('.domain').remove()).call(g=>g.selectAll('.tick line').attr('stroke','var(--border)'));
      const ticks=width<560?data.filter((_,i)=>i%Math.ceil(data.length/4)===0).slice(0,4).map(d=>d.start):data.map(d=>d.start);
      svg.append('g').attr('transform',`translate(0,${{height-m.bottom}})`).call(d3.axisBottom(x).tickValues(ticks).tickFormat(s=>{{const d=data.find(v=>v.start===s);return `${{d.start}}–${{d.end}}`;}})).call(g=>g.select('.domain').attr('stroke','var(--border)')).selectAll('text').attr('transform','rotate(-35)').style('text-anchor','end');
      svg.append('text').attr('class','axis-title').attr('data-axis','x').attr('x',m.left+iw/2).attr('y',height-8).attr('text-anchor','middle').text('入場順');
      svg.append('text').attr('class','axis-title').attr('data-axis','y').attr('transform',`translate(17,${{m.top+ih/2}}) rotate(-90)`).attr('text-anchor','middle').text('{config['axis']}');
      const baseline=y(domain[0]);
      const bars=svg.append('g').selectAll('rect').data(data).join('rect').attr('x',d=>x(d.start)).attr('y',d=>y(d.value)).attr('width',x.bandwidth()).attr('height',d=>baseline-y(d.value)).attr('fill','var(--viz-series-{config['series']})');
      svg.append('rect').attr('data-chart-hit','').attr('x',m.left).attr('y',m.top).attr('width',iw).attr('height',ih).attr('fill','transparent').on('pointermove',e=>{{const [px,py]=d3.pointer(e),i=Math.max(0,Math.min(data.length-1,Math.floor((px-m.left)/iw*data.length))),d=data[i];bars.attr('opacity',v=>v===d?1:.42);tooltip.style.display='block';tooltip.style.left=`${{Math.min(plot.clientWidth-210,Math.max(0,px+12))}}px`;tooltip.style.top=`${{Math.max(0,py-55)}}px`;tooltip.innerHTML=d.detail;}}).on('pointerleave',()=>{{bars.attr('opacity',1);tooltip.style.display='none';}});
    }}
    new ResizeObserver(draw).observe(plot);draw();
  }})();
  </script>
</div>
'''


def png_tick(value: float, value_format: str) -> str:
    if value_format == "percent":
        return f"{value * 100:.0f}%"
    if value_format == "fee":
        return f"{value / 1000:.0f}k"
    return f"{value:.3f}"


def render_png_svg(metrics: dict, config: dict) -> str:
    """Create a self-contained SVG suitable for ImageMagick PNG conversion."""
    width, height = 1200, 640
    left, right, top, bottom = 112, 40, 72, 132
    plot_width, plot_height = width - left - right, height - top - bottom
    values = [row[config["key"]] for row in metrics["bins"]]
    if not values or any(value is None for value in values):
        raise ValueError(f"{config['key']} is undefined for a bin")
    minimum, maximum = min(values), max(values)
    span = max(maximum - minimum, abs(maximum) * 0.05, 1e-9)
    y_min, y_max = (0.0, maximum + span * 0.12) if config["zero"] else (
        minimum - span * 0.14,
        maximum + span * 0.14,
    )
    if y_max <= y_min:
        y_max = y_min + 1.0

    def y(value: float) -> float:
        return top + (y_max - value) / (y_max - y_min) * plot_height

    parts = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="white"/>',
        f'<text x="{width / 2}" y="38" text-anchor="middle" font-family="{PNG_FONT}" font-size="24" font-weight="600">{config["png_title"]} (bins of {metrics["bin_size"]} groups)</text>',
        f'<rect x="{left}" y="{top}" width="{plot_width}" height="{plot_height}" fill="none" stroke="#777"/>',
    ]
    for index in range(6):
        value = y_min + (y_max - y_min) * index / 5
        py = y(value)
        parts.extend((
            f'<line x1="{left}" y1="{py:.2f}" x2="{width - right}" y2="{py:.2f}" stroke="#ddd"/>',
            f'<text x="{left - 12}" y="{py + 5:.2f}" text-anchor="end" font-family="{PNG_FONT}" font-size="16">{png_tick(value, config["format"])}</text>',
        ))
    bar_step = plot_width / len(values)
    bar_width = bar_step * 0.78
    baseline = y(y_min)
    for index, (row, value) in enumerate(zip(metrics["bins"], values)):
        x = left + index * bar_step + (bar_step - bar_width) / 2
        py = y(value)
        parts.append(
            f'<rect x="{x:.2f}" y="{py:.2f}" width="{bar_width:.2f}" height="{baseline - py:.2f}" fill="#4e79a7"/>'
        )
    tick_indices = list(range(len(values))) if len(values) <= 8 else [
        round(index * (len(values) - 1) / 3) for index in range(4)
    ]
    for index in dict.fromkeys(tick_indices):
        row = metrics["bins"][index]
        x = left + (index + 0.5) * bar_step
        parts.append(
            f'<text x="{x:.2f}" y="{height - bottom + 30}" text-anchor="end" transform="rotate(-35 {x:.2f} {height - bottom + 30})" font-family="{PNG_FONT}" font-size="15">{row["start"]}-{row["end"]}</text>'
        )
    parts.extend((
        f'<text x="{left + plot_width / 2}" y="{height - 24}" text-anchor="middle" font-family="{PNG_FONT}" font-size="18">Arrival order</text>',
        '</svg>',
    ))
    return "\n".join(parts)


def write_png(metrics: dict, config: dict, destination: Path) -> None:
    convert = shutil.which("convert") or shutil.which("magick")
    if convert is None:
        raise RuntimeError("PNG export requires ImageMagick ('convert' or 'magick') on PATH")
    command = [
        convert,
        "-background",
        "white",
        "-density",
        "144",
        "-encoding",
        "UTF-8",
        "svg:-",
        "png:-",
    ]
    environment = os.environ.copy()
    environment.setdefault("MAGICK_TMPDIR", str(destination.parent))
    result = subprocess.run(
        command,
        input=render_png_svg(metrics, config).encode("utf-8"),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=environment,
        check=False,
    )
    if result.returncode:
        message = result.stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"PNG export failed for {config['png_filename']}: {message}")
    destination.write_bytes(result.stdout)


def main() -> None:
    args = parse_args()
    metrics = json.loads(args.metrics.read_text())
    args.output_dir.mkdir(parents=True, exist_ok=True)
    html_paths = []
    png_paths = []
    for config in METRICS:
        destination = args.output_dir / config["filename"]
        destination.write_text(render_fragment(metrics, config))
        html_paths.append(str(destination.resolve()))
        png_destination = args.output_dir / config["png_filename"]
        write_png(metrics, config, png_destination)
        png_paths.append(str(png_destination.resolve()))
    print(json.dumps({"paths": html_paths, "png_paths": png_paths}, ensure_ascii=False))


if __name__ == "__main__":
    main()
