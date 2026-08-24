#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

python3 "$ROOT/scripts/generate-ntfs-resident-fixture.py" >/dev/null
SOURCE="$ROOT/fixtures/ntfs-deleted-resident-v1/source.img"
EXPECTED="$ROOT/fixtures/ntfs-deleted-resident-v1/expected-recovered.txt"
DIRECT_DESTINATION="$WORK/direct-destination"
SESSION_DESTINATION="$WORK/session-destination"
SESSION_MANIFEST="$WORK/evidenceforge-session.json"
mkdir -p "$DIRECT_DESTINATION" "$SESSION_DESTINATION"

EXPECTED_SHA256=$(sha256sum "$SOURCE" | awk '{print $1}')
grep -q "$EXPECTED_SHA256" "$ROOT/fixtures/ntfs-deleted-resident-v1/manifest.json"

cargo run -q -p ef-cli -- scan "$SOURCE" > "$WORK/scan.json"
ntfs_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$WORK/scan.json" --method ntfs_deleted_resident_record --source-offset 3264)
printf '%s\n' "$ntfs_id" | grep -q '^efc1-ntfs_deleted_resident_record-'
grep -q '"evidence_name": "gone.txt"' "$WORK/scan.json"
grep -q '"source_offset": 3264' "$WORK/scan.json"
grep -q '"method": "ntfs_deleted_resident_record"' "$WORK/scan.json"
grep -q '"validation": "recovered_unvalidated"' "$WORK/scan.json"

cargo run -q -p ef-cli -- catalogue "$SOURCE" --method ntfs > "$WORK/ntfs-catalogue.json"
grep -q "$ntfs_id" "$WORK/ntfs-catalogue.json"
grep -q 'does not recover non-resident data' "$WORK/ntfs-catalogue.json"

cargo run -q -p ef-cli -- recover "$SOURCE" "$ntfs_id" "$DIRECT_DESTINATION" > "$WORK/direct-export.json"
cmp "$EXPECTED" "$DIRECT_DESTINATION/$ntfs_id.txt"
grep -q '"source_range_start": 3264' "$WORK/direct-export.json"
grep -q '"recovery_method": "ntfs_deleted_resident_record"' "$WORK/direct-export.json"
grep -q '"validation": "recovered_unvalidated"' "$WORK/direct-export.json"

cargo run -q -p ef-cli -- save-session "$SOURCE" "$SESSION_MANIFEST" > "$WORK/session.json"
cargo run -q -p ef-cli -- session-status "$SESSION_MANIFEST" > "$WORK/session-status.json"
grep -q '"state": "verified"' "$WORK/session-status.json"
cargo run -q -p ef-cli -- recover-session "$SESSION_MANIFEST" "$ntfs_id" "$SESSION_DESTINATION" > "$WORK/session-export.json"
cmp "$EXPECTED" "$SESSION_DESTINATION/$ntfs_id.txt"
grep -q "$ntfs_id" "$SESSION_MANIFEST"

cargo test -q -p ef-fat ignores_a_record_with_an_invalid_fixup
cargo test -q -p ef-fat refuses_non_resident_data_attributes
cargo test -q -p ef-fat restores_resident_content_that_crosses_a_fixup_protected_sector_trailer

printf '%s\n' 'NTFS resident recovery verification passed'
