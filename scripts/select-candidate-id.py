#!/usr/bin/env python3
import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("scan_json", type=Path)
    parser.add_argument("--method", required=True)
    parser.add_argument("--file-type")
    parser.add_argument("--source-offset", type=lambda value: int(value, 0))
    arguments = parser.parse_args()

    with arguments.scan_json.open(encoding="utf-8") as source:
        scan = json.load(source)

    matches = []
    for candidate in scan.get("candidates", []):
        if candidate.get("method") != arguments.method:
            continue
        if arguments.file_type and candidate.get("file_type") != arguments.file_type:
            continue
        if (
            arguments.source_offset is not None
            and candidate.get("source_offset") != arguments.source_offset
        ):
            continue
        matches.append(candidate)

    if len(matches) != 1:
        print(
            f"expected exactly one candidate for method={arguments.method!r}, found {len(matches)}",
            file=sys.stderr,
        )
        return 1

    candidate_id = matches[0].get("id")
    if not isinstance(candidate_id, str) or not candidate_id.startswith("efc1-"):
        print("selected candidate does not have an EvidenceForge Candidate Identity v1 ID", file=sys.stderr)
        return 1
    print(candidate_id)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
