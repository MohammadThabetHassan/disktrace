#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
EVIDENCE_PATH=${1:?usage: sh scripts/verify-local-release-evidence.sh <evidence-json> <linux-archive> <windows-cross-target-archive>}
LINUX_ARCHIVE=${2:?usage: sh scripts/verify-local-release-evidence.sh <evidence-json> <linux-archive> <windows-cross-target-archive>}
WINDOWS_ARCHIVE=${3:?usage: sh scripts/verify-local-release-evidence.sh <evidence-json> <linux-archive> <windows-cross-target-archive>}

for command in basename sha256sum stat grep; do
    if ! command -v "$command" >/dev/null 2>&1; then
        printf '%s\n' "Local release-evidence verification requires: $command" >&2
        exit 1
    fi
done
for path in "$EVIDENCE_PATH" "$LINUX_ARCHIVE" "$WINDOWS_ARCHIVE"; do
    test -f "$path"
done

linux_name=$(basename "$LINUX_ARCHIVE")
windows_name=$(basename "$WINDOWS_ARCHIVE")
linux_hash=$(sha256sum "$LINUX_ARCHIVE" | awk '{print $1}')
windows_hash=$(sha256sum "$WINDOWS_ARCHIVE" | awk '{print $1}')
linux_size=$(stat -c '%s' "$LINUX_ARCHIVE")
windows_size=$(stat -c '%s' "$WINDOWS_ARCHIVE")

grep -q '"record_type": "local_verification_evidence"' "$EVIDENCE_PATH"
grep -q "\"name\": \"$linux_name\"" "$EVIDENCE_PATH"
grep -q "\"sha256\": \"$linux_hash\"" "$EVIDENCE_PATH"
grep -q "\"byte_size\": $linux_size" "$EVIDENCE_PATH"
grep -q "\"name\": \"$windows_name\"" "$EVIDENCE_PATH"
grep -q "\"sha256\": \"$windows_hash\"" "$EVIDENCE_PATH"
grep -q "\"byte_size\": $windows_size" "$EVIDENCE_PATH"
grep -q 'not a public release record' "$EVIDENCE_PATH"
grep -q 'not native Windows release evidence' "$EVIDENCE_PATH"

printf '%s\n' 'local release-evidence verification passed'
