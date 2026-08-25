#!/usr/bin/env python3
"""Render a narrow local evidence summary from validated scan-control CSV rows."""

from __future__ import annotations

import argparse
import csv
import statistics
from collections import defaultdict
from datetime import UTC, datetime
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize deterministic synthetic scan-control measurements."
    )
    parser.add_argument("input_csv", type=Path)
    parser.add_argument("output_markdown", type=Path)
    parser.add_argument(
        "--captured-at",
        default=datetime.now(UTC).strftime("%Y-%m-%d %H:%M:%SZ"),
        help="UTC evidence timestamp to record in the generated Markdown.",
    )
    return parser.parse_args()


def percentile_95(values: list[int]) -> int:
    ordered = sorted(values)
    index = max(0, (95 * len(ordered) + 99) // 100 - 1)
    return ordered[index]


def main() -> None:
    args = parse_args()
    scenarios: dict[str, dict[str, object]] = defaultdict(
        lambda: {"bytes": None, "expected_count": None, "offsets": None, "elapsed": []}
    )

    with args.input_csv.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        required = {
            "scenario",
            "bytes",
            "run",
            "elapsed_nanoseconds",
            "candidate_count",
            "expected_candidate_count",
            "expected_png_offsets",
        }
        if reader.fieldnames is None or set(reader.fieldnames) != required:
            raise SystemExit("scan-control CSV headers do not match the v1 contract")
        for row in reader:
            scenario = row["scenario"]
            candidate_count = int(row["candidate_count"])
            expected_count = int(row["expected_candidate_count"])
            if candidate_count != expected_count:
                raise SystemExit(f"{scenario} contains an invalid candidate-count row")
            record = scenarios[scenario]
            byte_count = int(row["bytes"])
            if record["bytes"] not in (None, byte_count):
                raise SystemExit(f"{scenario} contains inconsistent byte counts")
            if record["expected_count"] not in (None, expected_count):
                raise SystemExit(f"{scenario} contains inconsistent expected candidate counts")
            if record["offsets"] not in (None, row["expected_png_offsets"]):
                raise SystemExit(f"{scenario} contains inconsistent expected PNG offsets")
            record["bytes"] = byte_count
            record["expected_count"] = expected_count
            record["offsets"] = row["expected_png_offsets"]
            record["elapsed"].append(int(row["elapsed_nanoseconds"]))

    if not scenarios:
        raise SystemExit("scan-control CSV has no measurement rows")

    rows: list[str] = []
    for scenario in sorted(scenarios):
        record = scenarios[scenario]
        elapsed = record["elapsed"]
        assert isinstance(elapsed, list)
        mean_ns = statistics.fmean(elapsed)
        median_ns = statistics.median(elapsed)
        p95_ns = percentile_95(elapsed)
        byte_count = int(record["bytes"])
        mib_per_second = byte_count * 1_000_000_000 / mean_ns / (1024 * 1024)
        rows.append(
            "| {scenario} | {bytes:,} | {runs} | {count} | {offsets} | {median:.3f} | {p95:.3f} | {mean:.3f} | {throughput:.2f} |".format(
                scenario=scenario,
                bytes=byte_count,
                runs=len(elapsed),
                count=record["expected_count"],
                offsets=record["offsets"] or "none",
                median=median_ns / 1_000_000,
                p95=p95_ns / 1_000_000,
                mean=mean_ns / 1_000_000,
                throughput=mib_per_second,
            )
        )

    document = "\n".join(
        [
            "# Local synthetic scan-control corpus baseline v1",
            "",
            f"Captured locally at **{args.captured_at}** from `{args.input_csv.name}`.",
            "",
            "The harness generated each versioned synthetic source, asserted its expected candidate count and PNG offsets, then recorded current full-scan elapsed time. The exact scenarios and construction rules are defined in [`docs/performance-control-corpus-v1.md`](../docs/performance-control-corpus-v1.md).",
            "",
            "| Scenario | Bytes | Runs | Expected PNG candidates | Expected PNG offsets | Median ms | Observed p95 ms | Mean ms | Mean MiB/s |",
            "| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |",
            *rows,
            "",
            "## Interpretation boundary",
            "",
            "These measurements are deterministic synthetic regression observations for the current full-buffer scan path. They are not physical-device benchmarks, production throughput guarantees, fragmented-filesystem evidence, multi-gigabyte memory evidence, cache-state comparisons, malware-resilience evidence, or a claim that signature-dense sources recover more files. Each scenario includes only the declared synthetic byte pattern and expected candidate geometry.",
            "",
            "The current scan still identifies, hashes, and buffers the whole source before filesystem and structural discovery. PNG discovery separately enforces bounded-window parity, but this baseline does not establish whole-scan streaming, complete parser cancellation, or a resolved time-of-check/time-of-use boundary.",
            "",
        ]
    )
    args.output_markdown.parent.mkdir(parents=True, exist_ok=True)
    args.output_markdown.write_text(document, encoding="utf-8")


if __name__ == "__main__":
    main()
