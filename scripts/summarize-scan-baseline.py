#!/usr/bin/env python3
import argparse
import csv
import statistics
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    position = (len(ordered) - 1) * fraction
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    weight = position - lower
    return ordered[lower] * (1 - weight) + ordered[upper] * weight


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    arguments = parser.parse_args()

    grouped: dict[str, list[dict[str, int]]] = defaultdict(list)
    with arguments.input.open(newline="", encoding="utf-8") as source:
        for row in csv.DictReader(source):
            grouped[row["fixture"]].append(
                {
                    "bytes": int(row["bytes"]),
                    "elapsed_ns": int(row["elapsed_nanoseconds"]),
                    "candidate_count": int(row["candidate_count"]),
                }
            )

    rows = []
    for fixture, measurements in sorted(grouped.items()):
        elapsed_ms = [measurement["elapsed_ns"] / 1_000_000 for measurement in measurements]
        size = measurements[0]["bytes"]
        counts = {measurement["candidate_count"] for measurement in measurements}
        median_ms = statistics.median(elapsed_ms)
        mean_ms = statistics.fmean(elapsed_ms)
        throughput_mib_s = (size / (1024 * 1024)) / (mean_ms / 1000)
        rows.append(
            {
                "fixture": fixture,
                "bytes": size,
                "runs": len(measurements),
                "median_ms": median_ms,
                "p95_ms": percentile(elapsed_ms, 0.95),
                "mean_ms": mean_ms,
                "throughput_mib_s": throughput_mib_s,
                "candidate_counts": ", ".join(str(value) for value in sorted(counts)),
            }
        )

    captured_at = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%SZ")
    lines = [
        "# Local Scan Performance Baseline v1",
        "",
        f"Captured locally at **{captured_at}** from `{arguments.input.name}`.",
        "",
        "The harness runs the built CLI scan path repeatedly against the repository’s existing deterministic recovery fixtures. Each run performs current source inspection, SHA-256/BLAKE3 identity hashing, whole-image buffering, candidate discovery, and JSON serialization. No source is modified and no recovery output is created.",
        "",
        "| Fixture | Bytes | Runs | Candidate count(s) | Median ms | p95 ms | Mean ms | Mean MiB/s |",
        "| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |",
    ]
    for row in rows:
        lines.append(
            "| {fixture} | {bytes:,} | {runs} | {candidate_counts} | {median_ms:.3f} | {p95_ms:.3f} | {mean_ms:.3f} | {throughput_mib_s:.2f} |".format(
                **row
            )
        )

    largest = max(rows, key=lambda row: row["bytes"])
    lines.extend(
        [
            "",
            "## Interpretation boundary",
            "",
            f"The largest measured source is **{largest['bytes']:,} bytes** (`{largest['fixture']}`). These fixtures validate method behavior and regression consistency, but they are not a representative large-image benchmark. The figures therefore establish a repeatable small-fixture baseline and candidate-count stability only; they do not justify a streaming, memory-mapping, or signature-algorithm change.",
            "",
            "The current scan path still hashes and buffers the full image before parser discovery. Any future optimization must first add approved multi-gigabyte-shaped or signature-dense fixture evidence under the source-access migration contract, retain exact candidate and export comparisons, and compare like-for-like timings against this harness.",
            "",
        ]
    )
    arguments.output.write_text("\n".join(lines), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
