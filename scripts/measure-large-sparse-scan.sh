#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
RUNS=${1:-3}
OUTPUT=${2:-"$ROOT/local-verification/scan-performance-large-sparse-v1.csv"}
CLI="$ROOT/target/debug/evidenceforge"
OFFSET_BYTES=${OFFSET_BYTES:-33554432}
PNG_OFFSET=$((OFFSET_BYTES + 4096))
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

"$ROOT/scripts/generate-large-sparse-fixture.sh" "$WORK/source.img"
cargo build -q -p ef-cli --manifest-path "$ROOT/Cargo.toml"
printf '%s\n' 'fixture,bytes,run,elapsed_nanoseconds,candidate_count,expected_png_offset' > "$OUTPUT"

bytes=$(wc -c < "$WORK/source.img" | tr -d ' ')
run=1
while [ "$run" -le "$RUNS" ]; do
    json="$WORK/large-sparse-$run.json"
    started_at_ns=$(date +%s%N)
    "$CLI" scan "$WORK/source.img" > "$json"
    finished_at_ns=$(date +%s%N)
    elapsed_ns=$((finished_at_ns - started_at_ns))
    candidate_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$json" \
        --method signature_carving_png --file-type png --source-offset "$PNG_OFFSET")
    candidate_count=$(grep -c '^      "id": ' "$json" || true)
    case "$candidate_id" in
        efc1-signature_carving_png-*) ;;
        *)
            printf '%s\n' 'sparse fixture selected an unexpected candidate identity' >&2
            exit 1
            ;;
    esac
    printf '%s,%s,%s,%s,%s,%s\n' \
        'large-sparse-png-v1' "$bytes" "$run" "$elapsed_ns" "$candidate_count" "$PNG_OFFSET" >> "$OUTPUT"
    run=$((run + 1))
done

printf '%s\n' "large sparse scan baseline written to $OUTPUT"
