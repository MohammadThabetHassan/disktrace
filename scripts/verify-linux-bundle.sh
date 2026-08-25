#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    printf '%s\n' "usage: $0 <DiskTrace-linux-bundle.tar.gz>" >&2
    exit 2
fi

ARCHIVE=$1
if [ ! -f "$ARCHIVE" ]; then
    printf '%s\n' "bundle archive does not exist: $ARCHIVE" >&2
    exit 1
fi
if [ ! -f "$ARCHIVE.sha256" ]; then
    printf '%s\n' "bundle checksum does not exist: $ARCHIVE.sha256" >&2
    exit 1
fi

ARCHIVE_DIR=$(CDPATH= cd -- "$(dirname -- "$ARCHIVE")" && pwd)
ARCHIVE_NAME=$(basename -- "$ARCHIVE")
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

(
    cd "$ARCHIVE_DIR"
    sha256sum -c "$(basename -- "$ARCHIVE").sha256"
)

tar -xzf "$ARCHIVE" -C "$WORK"
BUNDLE_DIR=$(find "$WORK" -mindepth 1 -maxdepth 1 -type d -name 'disktrace-*-linux-x86_64' -print -quit)
if [ -z "$BUNDLE_DIR" ]; then
    printf '%s\n' "archive does not contain the expected Linux bundle root: $ARCHIVE_NAME" >&2
    exit 1
fi

for path in \
    bin/evidenceforge \
    bin/evidenceforge-desktop \
    launch-disktrace.sh \
    install-disktrace-launcher.sh \
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
    docs/linux-distribution-v1.md \
    docs/local-release-evidence-v1.md \
    docs/case-brief-v1.md \
    docs/future-github-launch-v1.md \
    docs/release-notes-v0.1.0-draft.md \
    release-manifest.json \
    SHA256SUMS; do
    test -s "$BUNDLE_DIR/$path"
done

test -x "$BUNDLE_DIR/bin/evidenceforge"
test -x "$BUNDLE_DIR/bin/evidenceforge-desktop"
test -x "$BUNDLE_DIR/launch-disktrace.sh"
test -x "$BUNDLE_DIR/install-disktrace-launcher.sh"
(
    cd "$BUNDLE_DIR"
    sha256sum -c SHA256SUMS
)
grep -q '"product": "DiskTrace"' "$BUNDLE_DIR/release-manifest.json"
grep -q '"source_commit": "[0-9a-f]\{40\}"' "$BUNDLE_DIR/release-manifest.json"
grep -q '"source_state": "clean-committed"' "$BUNDLE_DIR/release-manifest.json"
grep -q '"license": "Apache-2.0"' "$BUNDLE_DIR/release-manifest.json"
grep -q '"primary_launcher": "launch-disktrace.sh"' "$BUNDLE_DIR/release-manifest.json"
"$BUNDLE_DIR/bin/evidenceforge" --help >/dev/null

if command -v xvfb-run >/dev/null 2>&1; then
    set +e
    timeout 5s xvfb-run -a "$BUNDLE_DIR/launch-disktrace.sh" \
        > "$WORK/desktop.stdout" \
        2> "$WORK/desktop.stderr"
    status=$?
    set -e
    case "$status" in
        124)
            printf '%s\n' 'bundle desktop smoke launch passed'
            ;;
        0)
            printf '%s\n' 'bundle desktop smoke launch exited cleanly'
            ;;
        *)
            cat "$WORK/desktop.stderr" >&2
            exit "$status"
            ;;
    esac
else
    printf '%s\n' 'bundle desktop smoke launch skipped: xvfb-run is unavailable'
fi

printf '%s\n' 'Linux distribution bundle verification passed'
