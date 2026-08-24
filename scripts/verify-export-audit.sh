#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
WORK_ROOT=$(mktemp -d)
DESTINATION=$(mktemp -d)
cleanup() {
    rm -rf "$WORK_ROOT" "$DESTINATION"
}
trap cleanup EXIT

cd "$ROOT"
SOURCE="$WORK_ROOT/source.img"
MANIFEST="$WORK_ROOT/session.json"
AUDIT="$WORK_ROOT/audit.json"

cp fixtures/fat12-deleted-file-v1/source.img "$SOURCE"
cargo run -q -p ef-cli -- scan "$SOURCE" > "$WORK_ROOT/scan.json"
fat12_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$WORK_ROOT/scan.json" --method fat12_deleted_root_metadata --source-offset 1536)
OUTPUT="$DESTINATION/$fat12_id.txt"
cargo run -q -p ef-cli -- save-session "$SOURCE" "$MANIFEST" >/dev/null
cargo run -q -p ef-cli -- recover-session "$MANIFEST" "$fat12_id" "$DESTINATION" >/dev/null
cargo run -q -p ef-cli -- audit-session "$MANIFEST" > "$AUDIT"

grep -q '"source_integrity"' "$AUDIT"
grep -q '"state": "verified"' "$AUDIT"
grep -q '"exports"' "$AUDIT"

test -f "$OUTPUT"
printf '%s' 'changed recovered output' > "$OUTPUT"
cargo run -q -p ef-cli -- audit-session "$MANIFEST" > "$AUDIT"
grep -q '"state": "artifact_changed"' "$AUDIT"

printf '%s\n' 'receipt-backed export audit verification passed'
