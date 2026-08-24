#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
RUNS=${1:-15}
OUTPUT=${2:-"$ROOT/local-verification/scan-performance-baseline-v1.csv"}
CLI="$ROOT/target/debug/evidenceforge"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

case "$RUNS" in
    ''|*[!0-9]*)
        printf '%s\n' 'run count must be a positive integer' >&2
        exit 1
        ;;
esac

if [ "$RUNS" -eq 0 ]; then
    printf '%s\n' 'run count must be greater than zero' >&2
    exit 1
fi

cargo build -q -p ef-cli --manifest-path "$ROOT/Cargo.toml"
printf '%s\n' 'fixture,bytes,run,elapsed_nanoseconds,candidate_count' > "$OUTPUT"

for image in "$ROOT"/fixtures/*/source.img; do
    fixture=$(basename "$(dirname "$image")")
    bytes=$(wc -c < "$image" | tr -d ' ')
    run=1
    while [ "$run" -le "$RUNS" ]; do
        json="$WORK/$fixture-$run.json"
        started_at_ns=$(date +%s%N)
        "$CLI" scan "$image" > "$json"
        finished_at_ns=$(date +%s%N)
        elapsed_ns=$((finished_at_ns - started_at_ns))
        candidate_count=$(grep -c '^      "id": ' "$json" || true)
        printf '%s,%s,%s,%s,%s\n' \
            "$fixture" "$bytes" "$run" "$elapsed_ns" "$candidate_count" >> "$OUTPUT"
        run=$((run + 1))
    done
done

printf '%s\n' "scan baseline written to $OUTPUT"
