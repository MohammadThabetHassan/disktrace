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
BRIEF="$WORK_ROOT/case-brief.md"

cp fixtures/fat12-deleted-file-v1/source.img "$SOURCE"
cargo run -q -p ef-cli -- scan "$SOURCE" > "$WORK_ROOT/scan.json"
fat12_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$WORK_ROOT/scan.json" --method fat12_deleted_root_metadata --source-offset 1536)
cargo run -q -p ef-cli -- save-session "$SOURCE" "$MANIFEST" >/dev/null
cargo run -q -p ef-cli -- recover-session "$MANIFEST" "$fat12_id" "$DESTINATION" >/dev/null
cargo run -q -p ef-cli -- case-brief "$MANIFEST" "$BRIEF" >/dev/null

test -s "$BRIEF"
grep -q '^# DiskTrace case brief$' "$BRIEF"
grep -q 'Source SHA-256' "$BRIEF"
grep -q "$fat12_id" "$BRIEF"
grep -q 'Verified: persisted receipt and current SHA-256/BLAKE3 match' "$BRIEF"
if grep -q 'recover me' "$BRIEF"; then
    printf '%s\n' 'case brief unexpectedly contains recovered payload bytes' >&2
    exit 1
fi

printf '%s\n' 'local case brief verification passed'
