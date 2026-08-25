#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
ARCHIVE_PATH=${1:?usage: sh scripts/verify-windows-cross-target-bundle.sh <archive-path>}
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"

for command in awk sha256sum unzip wine xvfb-run timeout; do
    if ! command -v "$command" >/dev/null 2>&1; then
        printf '%s\n' "Windows cross-target bundle verification requires: $command" >&2
        exit 1
    fi
done
if [ ! -f "$ARCHIVE_PATH" ] || [ ! -f "$CHECKSUM_PATH" ]; then
    printf '%s\n' 'Windows cross-target bundle or checksum file is missing' >&2
    exit 1
fi

expected_archive_hash=$(awk 'NR == 1 { print $1 }' "$CHECKSUM_PATH")
actual_archive_hash=$(sha256sum "$ARCHIVE_PATH" | awk '{print $1}')
if [ "$expected_archive_hash" != "$actual_archive_hash" ]; then
    printf '%s\n' "Windows cross-target bundle checksum mismatch: $ARCHIVE_PATH" >&2
    exit 1
fi

WORK_DIRECTORY=$(mktemp -d)
cleanup() {
    rm -rf "$WORK_DIRECTORY"
}
trap cleanup EXIT
unzip -q "$ARCHIVE_PATH" -d "$WORK_DIRECTORY"
BUNDLE_DIRECTORY=$(find "$WORK_DIRECTORY" -mindepth 1 -maxdepth 1 -type d -name 'disktrace-*-windows-x86_64-cross-target' | head -n 1)
if [ -z "$BUNDLE_DIRECTORY" ]; then
    printf '%s\n' 'Windows cross-target bundle root is missing or malformed' >&2
    exit 1
fi

for relative_path in \
    bin/evidenceforge.exe \
    bin/evidenceforge-desktop.exe \
    docs/README.md \
    docs/LICENSE \
    docs/safety-and-evidence.md \
    docs/architecture.md \
    docs/candidate-identity-v1.md \
    docs/source-access-architecture-v1.md \
    docs/desktop-interaction-v2.md \
    docs/gui-workflow-v1.md \
    docs/release-process.md \
    docs/release-candidate-v0.1.0.md \
    docs/project-status.md \
    docs/dependency-advisories.md \
    docs/windows-distribution-v1.md \
    docs/local-release-evidence-v1.md \
    docs/case-brief-v1.md \
    docs/future-github-launch-v1.md \
    docs/release-notes-v0.1.0-draft.md \
    launch-evidenceforge.cmd \
    'Start DiskTrace.cmd' \
    release-manifest.json \
    SHA256SUMS; do
    test -f "$BUNDLE_DIRECTORY/$relative_path"
done

grep -q 'evidenceforge-desktop.exe' "$BUNDLE_DIRECTORY/Start DiskTrace.cmd"
grep -q '"product": "DiskTrace"' "$BUNDLE_DIRECTORY/release-manifest.json"
grep -q '"source_commit": "[0-9a-f]\{40\}"' "$BUNDLE_DIRECTORY/release-manifest.json"
grep -q '"source_state": "clean-committed"' "$BUNDLE_DIRECTORY/release-manifest.json"
grep -q '"artifact_evidence": "linux-cross-target-wine-compatibility"' "$BUNDLE_DIRECTORY/release-manifest.json"
grep -q '"target": "windows-x86_64"' "$BUNDLE_DIRECTORY/release-manifest.json"
grep -q 'Cross-target compatibility artifact, not native Windows release evidence' "$BUNDLE_DIRECTORY/release-manifest.json"

(
    cd "$BUNDLE_DIRECTORY"
    while IFS='  ' read -r expected_hash relative_path; do
        test -n "$expected_hash"
        test -n "$relative_path"
        actual_hash=$(sha256sum "$relative_path" | awk '{print $1}')
        if [ "$expected_hash" != "$actual_hash" ]; then
            printf '%s\n' "Windows cross-target staged-file checksum mismatch: $relative_path" >&2
            exit 1
        fi
    done < SHA256SUMS
)

help_output=$(mktemp)
desktop_stdout=$(mktemp)
desktop_stderr=$(mktemp)
cleanup_runtime() {
    rm -f "$help_output" "$desktop_stdout" "$desktop_stderr"
}
trap 'cleanup_runtime; cleanup' EXIT

set +e
WINEDEBUG=-all wine "$BUNDLE_DIRECTORY/bin/evidenceforge.exe" --help > "$help_output" 2>&1
set -e
grep -q 'evidenceforge audit-session <manifest-path>' "$help_output"

set +e
timeout 8s xvfb-run -a -s '-screen 0 1440x920x24' \
    env WINEDEBUG=-all wine "$BUNDLE_DIRECTORY/bin/evidenceforge-desktop.exe" \
    > "$desktop_stdout" 2> "$desktop_stderr"
status=$?
set -e
if [ "$status" -ne 124 ]; then
    cat "$desktop_stdout" "$desktop_stderr" >&2
    exit "$status"
fi
if [ -s "$desktop_stderr" ]; then
    cat "$desktop_stderr" >&2
    exit 1
fi

printf '%s\n' 'Windows cross-target portable bundle verification passed'
printf '%s\n' 'Boundary: this verifies archive integrity and Wine compatibility only; native Windows launcher, installer, signing, and usability evidence still require Windows.'
