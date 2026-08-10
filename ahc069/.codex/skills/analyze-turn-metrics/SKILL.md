---
name: analyze-turn-metrics
description: "Recompute and visualize AHC069 solution behavior by arrival-order bins from existing tools/in and tools/out files. Use when Codex needs the four recurring 50-turn charts or their underlying data: departure earnings, entry compactness, rejection rate, and pre-arrival vacancy rate; also use when comparing these metrics after an implementation change."
---

# Analyze AHC069 Turn Metrics

Analyze existing output logs only. Do not execute or modify the solution unless the user separately requests it. Respect the contest rule that results from a solution run must not trigger autonomous solution changes.

## Workflow

1. Confirm that no `pahcer` or tester run is currently updating `tools/out`. From the repository root, collect metrics:

   ```console
   python3 .codex/skills/analyze-turn-metrics/scripts/collect_turn_metrics.py \
     --input-dir tools/in --output-dir tools/out --bin-size 50 \
     --json-out /private/tmp/ahc069-turn-metrics.json
   ```

2. Save the canonical chart artifacts in the repository-local `.codex/turn-metrics` directory. The renderer saves both the four HTML fragments and matching PNG files there. Do not use a directory under the user's home-level `.codex` as the canonical output location.

   ```console
   python3 .codex/skills/analyze-turn-metrics/scripts/render_turn_metrics.py \
     /private/tmp/ahc069-turn-metrics.json --output-dir .codex/turn-metrics
   ```

3. Verify all four emitted HTML fragments with the visualization renderer. If the `visualize` skill is available and the charts should be shown inline, read and follow it; when its inline-display contract requires an external visualization directory, copy only the required HTML fragments there for display while retaining the canonical HTML and PNG files in `.codex/turn-metrics`. The matching PNG files are named `.codex/turn-metrics/arrival-*-by-turn-bin.png`; include these repository-local paths when the user requests image files. Then return all four visualization references in this order:

   1. Departure earnings
   2. Entry compactness
   3. Rejection rate
   4. Pre-arrival vacancy rate

4. State the number of cases and bin size. Keep interpretation to one short conclusion unless the user asks for analysis.

## Metric definitions

- **Departure earnings:** actual fee determined by the largest boundary experienced through departure. Average over every group in the bin; a group not admitted contributes zero.
- **Entry compactness:** `4 * sqrt(P) / L` for the initially allocated region. Average over admitted groups only.
- **Rejection rate:** groups answered `No` divided by all groups in the bin.
- **Pre-arrival vacancy rate:** free grass cells after processing earlier departures and before handling the arriving group, divided by all grass cells. Moves do not change this rate.

Use zero-based arrival order. With 100 cases and bin size 50, each full bin contains 5,000 groups.

## Variations

- Pass another positive `--bin-size` when requested.
- Pass repeated `--case` values such as `--case 0000 --case 0001` to restrict cases.
- Treat missing paired files, malformed logs, unexpected tokens, or incomplete bins as errors; do not silently omit them.
- If files change during collection, wait for the external evaluation to finish and rerun the collector. Never report a mixed snapshot.
