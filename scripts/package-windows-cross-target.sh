#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TARGET=x86_64-pc-windows-gnu
OUTPUT_DIRECTORY=${1:-"$ROOT/dist"}
mkdir -p "$OUTPUT_DIRECTORY"
OUTPUT_DIRECTORY=$(CDPATH= cd -- "$OUTPUT_DIRECTORY" && pwd)
VERSION=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$ROOT/Cargo.toml" | head -n 1)

if [ -z "$VERSION" ]; then
    printf '%s\n' 'Unable to determine workspace version from Cargo.toml' >&2
    exit 1
fi
if ! rustup target list --installed | grep -qx "$TARGET"; then
    printf '%s\n' "Windows cross-target packaging requires the Rust target: $TARGET" >&2
    exit 1
fi
for command in x86_64-w64-mingw32-gcc sha256sum zip; do
    if ! command -v "$command" >/dev/null 2>&1; then
        printf '%s\n' "Windows cross-target packaging requires: $command" >&2
        exit 1
    fi
done

BUNDLE_NAME="evidenceforge-$VERSION-windows-x86_64-cross-target"
ARCHIVE_NAME="EvidenceForge-$VERSION-windows-x86_64-cross-target.zip"
ARCHIVE_PATH="$OUTPUT_DIRECTORY/$ARCHIVE_NAME"
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"
STAGING_ROOT=$(mktemp -d)
BUNDLE_DIRECTORY="$STAGING_ROOT/$BUNDLE_NAME"
cleanup() {
    rm -rf "$STAGING_ROOT"
}
trap cleanup EXIT

mkdir -p "$BUNDLE_DIRECTORY/bin" "$BUNDLE_DIRECTORY/docs"
cd "$ROOT"
cargo build --release --target "$TARGET" -p ef-cli -p ef-desktop

cp "target/$TARGET/release/evidenceforge.exe" "$BUNDLE_DIRECTORY/bin/evidenceforge.exe"
cp "target/$TARGET/release/evidenceforge-desktop.exe" "$BUNDLE_DIRECTORY/bin/evidenceforge-desktop.exe"
for document in \
    README.md \
    LICENSE \
    docs/safety-and-evidence.md \
    docs/architecture.md \
    docs/candidate-identity-v1.md \
    docs/source-access-architecture-v1.md \
    docs/desktop-interaction-v2.md \
    docs/gui-workflow-v1.md \
    docs/release-process.md \
    docs/dependency-advisories.md \
    docs/windows-distribution-v1.md \
    docs/local-release-evidence-v1.md \
    docs/case-brief-v1.md \
    docs/future-github-launch-v1.md \
    docs/release-notes-v0.1.0-draft.md; do
    cp "$document" "$BUNDLE_DIRECTORY/docs/$(basename "$document")"
done

cat > "$BUNDLE_DIRECTORY/launch-evidenceforge.cmd" <<'EOF'
@echo off
setlocal
start "" "%~dp0bin\evidenceforge-desktop.exe" %*
EOF
cp "$BUNDLE_DIRECTORY/launch-evidenceforge.cmd" "$BUNDLE_DIRECTORY/Start EvidenceForge Recovery.cmd"

cat > "$BUNDLE_DIRECTORY/release-manifest.json" <<EOF
{
  "schema_version": 1,
  "product": "EvidenceForge Recovery",
  "version": "$VERSION",
  "target": "windows-x86_64",
  "format": "zip",
  "license": "Apache-2.0",
  "source_state": "local-uncommitted",
  "artifact_evidence": "linux-cross-target-wine-compatibility",
  "supported_build_host": "Linux x86_64 cross-target environment",
  "primary_launcher": "Start EvidenceForge Recovery.cmd",
  "included_binaries": [
    "bin/evidenceforge-desktop.exe",
    "bin/evidenceforge.exe"
  ],
  "build_evidence": {
    "rust_target": "$TARGET",
    "build_command": "cargo build --release --target $TARGET -p ef-cli -p ef-desktop",
    "cli_smoke": "Wine command-surface verification required",
    "desktop_smoke": "Wine/Xvfb bounded process verification required"
  },
  "intentional_limits": [
    "Cross-target compatibility artifact, not native Windows release evidence",
    "Not Authenticode signed",
    "Not an MSI, Microsoft Store package, or automatic update channel",
    "Not validated for Windows on ARM",
    "Not a public release artifact"
  ]
}
EOF

(
    cd "$BUNDLE_DIRECTORY"
    {
        find bin docs -type f -print
        printf '%s\n' launch-evidenceforge.cmd
        printf '%s\n' 'Start EvidenceForge Recovery.cmd'
        printf '%s\n' release-manifest.json
    } | LC_ALL=C sort | while IFS= read -r relative_path; do
        hash=$(sha256sum "$relative_path" | awk '{print $1}')
        printf '%s  %s\n' "$hash" "$relative_path"
    done > SHA256SUMS
)

rm -f "$ARCHIVE_PATH" "$CHECKSUM_PATH"
(
    cd "$STAGING_ROOT"
    zip -q -r "$ARCHIVE_PATH" "$BUNDLE_NAME"
)
archive_hash=$(sha256sum "$ARCHIVE_PATH" | awk '{print $1}')
printf '%s  %s\n' "$archive_hash" "$ARCHIVE_NAME" > "$CHECKSUM_PATH"

printf '%s\n' "created $ARCHIVE_PATH"
printf '%s\n' "created $CHECKSUM_PATH"
