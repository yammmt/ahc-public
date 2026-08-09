#!/usr/bin/env python3
"""Collect four arrival-order metrics from AHC069 input/output pairs."""

from __future__ import annotations

import argparse
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


@dataclass(frozen=True)
class Group:
    index: int
    group_id: int
    arrival: int
    departure: int
    size: int
    value: int


@dataclass
class ActiveGroup:
    group: Group
    maximum_boundary: int


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--bin-size", type=int, default=50)
    parser.add_argument("--case", action="append", dest="cases")
    parser.add_argument("--json-out", type=Path)
    args = parser.parse_args()
    if args.bin_size <= 0:
        parser.error("--bin-size must be positive")
    return args


def tokens_without_comments(path: Path) -> list[str]:
    tokens: list[str] = []
    for line in path.read_text().splitlines():
        if not line.lstrip().startswith("#"):
            tokens.extend(line.split())
    return tokens


def boundary_length(cells: Iterable[tuple[int, int]]) -> int:
    region = set(cells)
    return sum(
        (x + dx, y + dy) not in region
        for x, y in region
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
    )


def round_payment(value: int, size: int, boundary: int) -> int:
    if boundary <= 0:
        return 0
    squared = 64 * value * value * size

    def lower_ok(fee: int) -> bool:
        threshold = (2 * fee - 1) * boundary
        return threshold <= 0 or threshold * threshold <= squared

    def upper_ok(fee: int) -> bool:
        threshold = (2 * fee + 1) * boundary
        return squared < threshold * threshold

    fee = int(value * 4.0 * math.sqrt(size) / boundary + 0.5)
    while not lower_ok(fee):
        fee -= 1
    while not upper_ok(fee):
        fee += 1
    return fee


def read_input(path: Path) -> tuple[int, list[Group]]:
    lines = path.read_text().splitlines()
    n, m, _ = lines[0].split()
    side = int(n)
    grass_area = sum(cell == "." for row in lines[1 : side + 1] for cell in row.strip())
    groups = []
    for index, line in enumerate(lines[side + 1 : side + 1 + int(m)]):
        group_id, arrival, departure, size, value = map(int, line.split())
        groups.append(Group(index, group_id, arrival, departure, size, value))
    if len(groups) != int(m):
        raise ValueError(f"{path}: expected {m} groups, found {len(groups)}")
    return grass_area, groups


def consume_cells(tokens: list[str], position: int, count: int) -> tuple[list[tuple[int, int]], int]:
    end = position + 2 * count
    if end > len(tokens):
        raise ValueError("output ended while reading a region")
    cells = [(int(tokens[i]), int(tokens[i + 1])) for i in range(position, end, 2)]
    return cells, end


def analyze_case(input_path: Path, output_path: Path) -> list[dict[str, float | bool]]:
    grass_area, groups = read_input(input_path)
    by_id = {group.group_id: group for group in groups}
    tokens = tokens_without_comments(output_path)
    position = 0
    used_area = 0
    active: dict[int, ActiveGroup] = {}
    fees = [0] * len(groups)
    observations: list[dict[str, float | bool]] = []

    def depart(group_id: int) -> None:
        nonlocal used_area
        active_group = active.pop(group_id)
        group = active_group.group
        fees[group.index] = round_payment(group.value, group.size, active_group.maximum_boundary)
        used_area -= group.size

    for group in groups:
        for group_id, active_group in list(active.items()):
            if active_group.group.departure < group.arrival:
                depart(group_id)

        vacancy_rate = (grass_area - used_area) / grass_area
        if position >= len(tokens):
            raise ValueError(f"{output_path}: output ended before group {group.index}")
        move_count = int(tokens[position])
        position += 1
        for _ in range(move_count):
            group_id = int(tokens[position])
            position += 1
            if group_id not in active or group_id not in by_id:
                raise ValueError(f"{output_path}: moved inactive group {group_id}")
            cells, position = consume_cells(tokens, position, by_id[group_id].size)
            active[group_id].maximum_boundary = max(
                active[group_id].maximum_boundary, boundary_length(cells)
            )

        if position >= len(tokens):
            raise ValueError(f"{output_path}: missing Yes/No for group {group.index}")
        decision = tokens[position]
        position += 1
        if decision == "Yes":
            cells, position = consume_cells(tokens, position, group.size)
            boundary = boundary_length(cells)
            active[group.group_id] = ActiveGroup(group, boundary)
            used_area += group.size
            compactness = 4.0 * math.sqrt(group.size) / boundary
            admitted = True
        elif decision == "No":
            compactness = 0.0
            admitted = False
        else:
            raise ValueError(f"{output_path}: expected Yes/No, found {decision!r}")
        observations.append(
            {"admitted": admitted, "compactness": compactness, "vacancy_rate": vacancy_rate}
        )

    for group_id in list(active):
        depart(group_id)
    if position != len(tokens):
        raise ValueError(f"{output_path}: {len(tokens) - position} unexpected trailing tokens")
    for observation, fee in zip(observations, fees):
        observation["fee"] = fee
    return observations


def main() -> None:
    args = parse_args()
    if args.cases:
        case_names = [case.removesuffix(".txt") for case in args.cases]
    else:
        case_names = sorted(path.stem for path in args.input_dir.glob("*.txt"))
    if not case_names:
        raise ValueError("no input cases found")

    paired_paths = []
    all_cases = []
    for case_name in case_names:
        input_path = args.input_dir / f"{case_name}.txt"
        output_path = args.output_dir / f"{case_name}.txt"
        if not input_path.is_file() or not output_path.is_file():
            raise FileNotFoundError(f"missing input/output pair for case {case_name}")
        paired_paths.extend((input_path, output_path))
    initial_stats = {path: (path.stat().st_mtime_ns, path.stat().st_size) for path in paired_paths}
    for case_name in case_names:
        input_path = args.input_dir / f"{case_name}.txt"
        output_path = args.output_dir / f"{case_name}.txt"
        all_cases.append(analyze_case(input_path, output_path))
    final_stats = {path: (path.stat().st_mtime_ns, path.stat().st_size) for path in paired_paths}
    changed = [path for path in paired_paths if initial_stats[path] != final_stats[path]]
    if changed:
        names = ", ".join(str(path) for path in changed[:5])
        raise RuntimeError(f"input/output files changed during collection: {names}")

    group_count = len(all_cases[0])
    if any(len(case) != group_count for case in all_cases):
        raise ValueError("cases have different group counts")
    bins = []
    for start in range(0, group_count, args.bin_size):
        end = min(start + args.bin_size, group_count)
        observations = [case[index] for case in all_cases for index in range(start, end)]
        admitted = [observation for observation in observations if observation["admitted"]]
        total_fee = sum(int(observation["fee"]) for observation in observations)
        bins.append(
            {
                "start": start,
                "end": end - 1,
                "group_count": len(observations),
                "admitted_count": len(admitted),
                "rejected_count": len(observations) - len(admitted),
                "total_fee": total_fee,
                "mean_fee": total_fee / len(observations),
                "mean_entry_compactness": (
                    sum(float(observation["compactness"]) for observation in admitted)
                    / len(admitted)
                    if admitted
                    else None
                ),
                "rejection_rate": 1.0 - len(admitted) / len(observations),
                "mean_pre_arrival_vacancy_rate": sum(
                    float(observation["vacancy_rate"]) for observation in observations
                )
                / len(observations),
            }
        )

    result = {
        "schema_version": 1,
        "case_count": len(case_names),
        "group_count_per_case": group_count,
        "bin_size": args.bin_size,
        "cases": case_names,
        "bins": bins,
    }
    serialized = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    if args.json_out:
        args.json_out.write_text(serialized)
    else:
        print(serialized, end="")


if __name__ == "__main__":
    main()
