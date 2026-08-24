#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
OFFSET_BYTES=${OFFSET_BYTES:-33554432}
EXPECTED_PNG_OFFSET=$((OFFSET_BYTES + 4096))
EXPECTED_BYTES=${TOTAL_BYTES:-67108864}

"$ROOT/scripts/generate-large-sparse-fixture.sh" "$WORK/source.img" > "$WORK/generate.log"
test "$(wc -c < "$WORK/source.img" | tr -d ' ')" = "$EXPECTED_BYTES"

cargo build -q -p ef-cli --manifest-path "$ROOT/Cargo.toml"
"$ROOT/target/debug/evidenceforge" scan "$WORK/source.img" > "$WORK/scan.json"
python3 "$ROOT/scripts/select-candidate-id.py" "$WORK/scan.json" \
    --method signature_carving_png --file-type png --source-offset "$EXPECTED_PNG_OFFSET" \
    > "$WORK/candidate-id"
grep -q '^efc1-signature_carving_png-' "$WORK/candidate-id"
test "$(grep -c '^      "id": ' "$WORK/scan.json" || true)" = 1

printf '%s\n' 'large sparse source-control verification passed'
