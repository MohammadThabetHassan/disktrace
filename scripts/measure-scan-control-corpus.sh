#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
RUNS=${1:-3}
OUTPUT=${2:-"$ROOT/local-verification/scan-performance-control-corpus-v1.csv"}
CLI="$ROOT/target/debug/evidenceforge"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT HUP INT TERM

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

scenario_value() {
    description=$1
    key=$2
    printf '%s\n' "$description" | sed -n "s/^$key=//p"
}

verify_expected_candidates() {
    scan_json=$1
    expected_count=$2
    expected_offsets=$3

    candidate_count=$(grep -c '^      "id": ' "$scan_json" || true)
    test "$candidate_count" = "$expected_count"

    if [ -n "$expected_offsets" ]; then
        old_ifs=$IFS
        IFS=,
        set -- $expected_offsets
        IFS=$old_ifs
        for source_offset in "$@"; do
            candidate_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$scan_json" \
                --method signature_carving_png --file-type png --source-offset "$source_offset")
            case "$candidate_id" in
                efc1-signature_carving_png-*) ;;
                *)
                    printf '%s\n' 'scan-control corpus selected an unexpected candidate identity' >&2
                    exit 1
                    ;;
            esac
        done
    fi

    printf '%s\n' "$candidate_count"
}

mkdir -p "$(dirname -- "$OUTPUT")"
printf '%s\n' 'scenario,bytes,run,elapsed_nanoseconds,candidate_count,expected_candidate_count,expected_png_offsets' > "$OUTPUT"
cargo build -q -p ef-cli --manifest-path "$ROOT/Cargo.toml"

for scenario in \
    large-sparse-png-v1 \
    signature-dense-png-v1 \
    signature-dense-refusal-v1 \
    multi-candidate-png-v1; do
    description=$("$ROOT/scripts/generate-scan-control-fixture.sh" --describe "$scenario")
    expected_bytes=$(scenario_value "$description" total_bytes)
    expected_count=$(scenario_value "$description" expected_png_count)
    expected_offsets=$(scenario_value "$description" expected_png_offsets)
    source="$WORK/$scenario.img"

    "$ROOT/scripts/generate-scan-control-fixture.sh" "$scenario" "$source" > "$WORK/$scenario.generate.log"
    test "$(wc -c < "$source" | tr -d ' ')" = "$expected_bytes"

    run=1
    while [ "$run" -le "$RUNS" ]; do
        scan_json="$WORK/$scenario-$run.json"
        started_at_ns=$(date +%s%N)
        "$CLI" scan "$source" > "$scan_json"
        finished_at_ns=$(date +%s%N)
        elapsed_ns=$((finished_at_ns - started_at_ns))
        candidate_count=$(verify_expected_candidates "$scan_json" "$expected_count" "$expected_offsets")
        printf '%s,%s,%s,%s,%s,%s,"%s"\n' \
            "$scenario" "$expected_bytes" "$run" "$elapsed_ns" "$candidate_count" "$expected_count" "$expected_offsets" >> "$OUTPUT"
        run=$((run + 1))
    done
done

printf '%s\n' "synthetic scan-control corpus baseline written to $OUTPUT"
