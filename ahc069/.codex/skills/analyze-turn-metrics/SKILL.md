---
name: analyze-turn-metrics
description: Recompute and visualize AHC069 solution behavior by arrival-order bins from existing tools/in and tools/out files. Use when Codex needs the four recurring 50-turn charts or their underlying data: departure earnings, entry compactness, rejection rate, and pre-arrival vacancy rate; also use when comparing these metrics after an implementation change.
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

2. For conversational charts, use a durable writable visualization directory outside the repository. If the `visualize` skill is available, read and follow it before rendering or returning the charts.

   ```console
   python3 .codex/skills/analyze-turn-metrics/scripts/render_turn_metrics.py \
     /private/tmp/ahc069-turn-metrics.json --output-dir <visualization-directory>
   ```

3. Verify all four emitted HTML fragments with the visualization renderer, then return all four visualization references in this order:

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
