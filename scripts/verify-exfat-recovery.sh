#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

python3 "$ROOT/scripts/generate-exfat-fixture.py" >/dev/null
SOURCE="$ROOT/fixtures/exfat-contiguous-deleted-v1/source.img"
EXPECTED="$ROOT/fixtures/exfat-contiguous-deleted-v1/expected-recovered.txt"
DIRECT_DESTINATION="$WORK/direct-destination"
SESSION_DESTINATION="$WORK/session-destination"
SESSION_MANIFEST="$WORK/evidenceforge-session.json"
mkdir -p "$DIRECT_DESTINATION" "$SESSION_DESTINATION"

EXPECTED_SHA256=$(sha256sum "$SOURCE" | awk '{print $1}')
grep -q "$EXPECTED_SHA256" "$ROOT/fixtures/exfat-contiguous-deleted-v1/manifest.json"

cargo run -q -p ef-cli -- scan "$SOURCE" > "$WORK/scan.json"
exfat_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$WORK/scan.json" --method exfat_deleted_contiguous_root_metadata --source-offset 21504)
printf '%s\n' "$exfat_id" | grep -q '^efc1-exfat_deleted_contiguous_root_metadata-'
grep -q '"evidence_name": "recover.txt"' "$WORK/scan.json"
grep -q '"source_offset": 21504' "$WORK/scan.json"
grep -q '"method": "exfat_deleted_contiguous_root_metadata"' "$WORK/scan.json"
grep -q '"validation": "recovered_unvalidated"' "$WORK/scan.json"

cargo run -q -p ef-cli -- catalogue "$SOURCE" --method exfat > "$WORK/exfat-catalogue.json"
grep -q "$exfat_id" "$WORK/exfat-catalogue.json"
grep -q 'active allocation bitmap reports as free' "$WORK/exfat-catalogue.json"

cargo run -q -p ef-cli -- recover "$SOURCE" "$exfat_id" "$DIRECT_DESTINATION" > "$WORK/direct-export.json"
cmp "$EXPECTED" "$DIRECT_DESTINATION/$exfat_id.txt"
grep -q '"source_range_start": 21504' "$WORK/direct-export.json"
grep -q '"recovery_method": "exfat_deleted_contiguous_root_metadata"' "$WORK/direct-export.json"
grep -q '"validation": "recovered_unvalidated"' "$WORK/direct-export.json"

cargo run -q -p ef-cli -- save-session "$SOURCE" "$SESSION_MANIFEST" > "$WORK/session.json"
cargo run -q -p ef-cli -- session-status "$SESSION_MANIFEST" > "$WORK/session-status.json"
grep -q '"state": "verified"' "$WORK/session-status.json"
cargo run -q -p ef-cli -- recover-session "$SESSION_MANIFEST" "$exfat_id" "$SESSION_DESTINATION" > "$WORK/session-export.json"
cmp "$EXPECTED" "$SESSION_DESTINATION/$exfat_id.txt"
grep -q "$exfat_id" "$SESSION_MANIFEST"

cargo test -q -p ef-fat rejects_an_invalid_main_boot_checksum
cargo test -q -p ef-fat ignores_a_deleted_entry_set_when_a_content_cluster_is_allocated

printf '%s\n' 'exFAT recovery verification passed'
