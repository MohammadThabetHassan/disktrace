#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

python3 "$ROOT/scripts/generate-fat12-fixture.py" >/dev/null
mkdir -p "$WORK/source" "$WORK/destination"
cp "$ROOT/fixtures/fat12-deleted-file-v1/source.img" "$WORK/source/source.img"

SOURCE="$WORK/source/source.img"
DESTINATION="$WORK/destination"
MANIFEST="$WORK/evidenceforge-session.json"

cargo run -q -p ef-cli -- scan "$SOURCE" > "$WORK/scan.json"
fat12_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$WORK/scan.json" --method fat12_deleted_root_metadata --source-offset 1536)
png_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$WORK/scan.json" --method signature_carving_png --source-offset 4096)
cargo run -q -p ef-cli -- save-session "$SOURCE" "$MANIFEST" > "$WORK/saved-session.json"
grep -q '"schema_version": 1' "$WORK/saved-session.json"
grep -q '"status": "scan_completed"' "$WORK/saved-session.json"
grep -q "$fat12_id" "$WORK/saved-session.json"
grep -q "$png_id" "$WORK/saved-session.json"

cargo run -q -p ef-cli -- session-status "$MANIFEST" > "$WORK/verified-status.json"
grep -q '"state": "verified"' "$WORK/verified-status.json"

cargo run -q -p ef-cli -- recover-session "$MANIFEST" "$fat12_id" "$DESTINATION" > "$WORK/export.json"
cmp "$ROOT/fixtures/fat12-deleted-file-v1/expected-recovered.txt" "$DESTINATION/$fat12_id.txt"
grep -q '"session_id"' "$WORK/export.json"
grep -q "$fat12_id" "$MANIFEST"
grep -q '"exports": \[' "$MANIFEST"

printf 'changed source bytes\n' > "$SOURCE"
cargo run -q -p ef-cli -- session-status "$MANIFEST" > "$WORK/changed-status.json"
grep -q '"state": "changed"' "$WORK/changed-status.json"
if cargo run -q -p ef-cli -- recover-session "$MANIFEST" "$png_id" "$DESTINATION" >"$WORK/changed-recovery.out" 2>"$WORK/changed-recovery.err"; then
    printf '%s\n' 'changed source recovery unexpectedly succeeded' >&2
    exit 1
fi
grep -q 'recorded source image changed' "$WORK/changed-recovery.err"

printf '%s\n' 'session persistence verification passed'
