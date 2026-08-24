#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

python3 "$ROOT/scripts/generate-ntfs-contiguous-fixture.py" >/dev/null
SOURCE="$ROOT/fixtures/ntfs-deleted-contiguous-v1/source.img"
EXPECTED="$ROOT/fixtures/ntfs-deleted-contiguous-v1/expected-recovered.txt"
DIRECT_DESTINATION="$WORK/direct-destination"
SESSION_DESTINATION="$WORK/session-destination"
SESSION_MANIFEST="$WORK/evidenceforge-session.json"
REALLOCATED="$WORK/reallocated.img"
mkdir -p "$DIRECT_DESTINATION" "$SESSION_DESTINATION"

EXPECTED_SHA256=$(sha256sum "$SOURCE" | awk '{print $1}')
grep -q "$EXPECTED_SHA256" "$ROOT/fixtures/ntfs-deleted-contiguous-v1/manifest.json"

cargo run -q -p ef-cli -- scan "$SOURCE" > "$WORK/scan.json"
ntfs_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$WORK/scan.json" --method ntfs_deleted_contiguous_nonresident --source-offset 32768)
printf '%s\n' "$ntfs_id" | grep -q '^efc1-ntfs_deleted_contiguous_nonresident-'
grep -q '"evidence_name": "extent.txt"' "$WORK/scan.json"
grep -q '"source_offset": 32768' "$WORK/scan.json"
grep -q '"byte_length": 16' "$WORK/scan.json"
grep -q '"method": "ntfs_deleted_contiguous_nonresident"' "$WORK/scan.json"
grep -q '"validation": "recovered_unvalidated"' "$WORK/scan.json"

cargo run -q -p ef-cli -- catalogue "$SOURCE" --method ntfs-contiguous > "$WORK/catalogue.json"
grep -q "$ntfs_id" "$WORK/catalogue.json"
grep -q 'current allocation bitmap' "$WORK/catalogue.json"
grep -q 'does not prove the former bytes were not overwritten' "$WORK/catalogue.json"

cargo run -q -p ef-cli -- recover "$SOURCE" "$ntfs_id" "$DIRECT_DESTINATION" > "$WORK/direct-export.json"
cmp "$EXPECTED" "$DIRECT_DESTINATION/$ntfs_id.txt"
grep -q '"source_range_start": 32768' "$WORK/direct-export.json"
grep -q '"recovery_method": "ntfs_deleted_contiguous_nonresident"' "$WORK/direct-export.json"
grep -q '"validation": "recovered_unvalidated"' "$WORK/direct-export.json"

cargo run -q -p ef-cli -- save-session "$SOURCE" "$SESSION_MANIFEST" > "$WORK/session.json"
cargo run -q -p ef-cli -- session-status "$SESSION_MANIFEST" > "$WORK/session-status.json"
grep -q '"state": "verified"' "$WORK/session-status.json"
cargo run -q -p ef-cli -- recover-session "$SESSION_MANIFEST" "$ntfs_id" "$SESSION_DESTINATION" > "$WORK/session-export.json"
cmp "$EXPECTED" "$SESSION_DESTINATION/$ntfs_id.txt"
grep -q "$ntfs_id" "$SESSION_MANIFEST"

cp "$SOURCE" "$REALLOCATED"
printf '\001' | dd of="$REALLOCATED" bs=1 seek=$((2048 + 6 * 1024 + 56 + 24 + 8)) conv=notrunc status=none
cargo run -q -p ef-cli -- scan "$REALLOCATED" > "$WORK/reallocated-scan.json"
if python3 "$ROOT/scripts/select-candidate-id.py" "$WORK/reallocated-scan.json" --method ntfs_deleted_contiguous_nonresident --source-offset 32768 >/dev/null 2>&1; then
    printf '%s\n' 'allocated NTFS extent was incorrectly offered for recovery' >&2
    exit 1
fi

cargo test -q -p ef-fat recovers_a_deleted_nonresident_single_run_when_its_clusters_are_free
cargo test -q -p ef-fat ignores_a_nonresident_candidate_after_its_cluster_is_reallocated
cargo test -q -p ef-fat refuses_a_nonresident_record_with_an_unterminated_second_run

printf '%s\n' 'NTFS contiguous recovery verification passed'
