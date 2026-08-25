#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
CLI="$ROOT/target/debug/evidenceforge"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT HUP INT TERM

scenario_value() {
    description=$1
    key=$2
    printf '%s\n' "$description" | sed -n "s/^$key=//p"
}

verify_scenario() {
    scenario=$1
    description=$("$ROOT/scripts/generate-scan-control-fixture.sh" --describe "$scenario")
    expected_bytes=$(scenario_value "$description" total_bytes)
    expected_count=$(scenario_value "$description" expected_png_count)
    expected_offsets=$(scenario_value "$description" expected_png_offsets)
    source="$WORK/$scenario.img"
    scan_json="$WORK/$scenario.json"

    "$ROOT/scripts/generate-scan-control-fixture.sh" "$scenario" "$source" > "$WORK/$scenario.generate.log"
    test "$(wc -c < "$source" | tr -d ' ')" = "$expected_bytes"
    "$CLI" scan "$source" > "$scan_json"

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
                    printf '%s\n' "$scenario selected an unexpected candidate identity" >&2
                    exit 1
                    ;;
            esac
        done
    fi

    printf '%s\n' "$scenario source-control verification passed"
}

cargo build -q -p ef-cli --manifest-path "$ROOT/Cargo.toml"
for scenario in \
    large-sparse-png-v1 \
    signature-dense-png-v1 \
    signature-dense-refusal-v1 \
    multi-candidate-png-v1; do
    verify_scenario "$scenario"
done

printf '%s\n' 'synthetic scan-control corpus verification passed'
