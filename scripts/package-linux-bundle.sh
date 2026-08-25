#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUTPUT_DIR=${1:-"$ROOT/dist"}
mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR=$(CDPATH= cd -- "$OUTPUT_DIR" && pwd)

if [ "$(uname -s)" != "Linux" ]; then
    printf '%s\n' 'Linux bundle creation must run on a Linux host' >&2
    exit 1
fi
if [ "$(uname -m)" != "x86_64" ]; then
    printf '%s\n' 'Linux bundle creation currently supports only x86_64 hosts' >&2
    exit 1
fi

VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)
if [ -z "$VERSION" ]; then
    printf '%s\n' 'unable to determine workspace version' >&2
    exit 1
fi

TARGET=linux-x86_64
cd "$ROOT"
if ! git diff --quiet || ! git diff --cached --quiet || [ -n "$(git ls-files --others --exclude-standard)" ]; then
    printf '%s\n' 'Linux bundle creation requires a clean committed source revision.' >&2
    exit 1
fi
SOURCE_COMMIT=$(git rev-parse HEAD)
BUNDLE_NAME="disktrace-${VERSION}-${TARGET}"
ARCHIVE_NAME="DiskTrace-${VERSION}-${TARGET}.tar.gz"
ARCHIVE_PATH="$OUTPUT_DIR/$ARCHIVE_NAME"
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"
STAGING_ROOT=$(mktemp -d)
BUNDLE_DIR="$STAGING_ROOT/$BUNDLE_NAME"
trap 'rm -rf "$STAGING_ROOT"' EXIT

mkdir -p "$BUNDLE_DIR/bin" "$BUNDLE_DIR/docs"

cargo build --release -p ef-cli -p ef-desktop

install -m 0755 "$ROOT/target/release/evidenceforge" "$BUNDLE_DIR/bin/evidenceforge"
install -m 0755 "$ROOT/target/release/evidenceforge-desktop" "$BUNDLE_DIR/bin/evidenceforge-desktop"
install -m 0644 "$ROOT/README.md" "$BUNDLE_DIR/docs/README.md"
install -m 0644 "$ROOT/LICENSE" "$BUNDLE_DIR/docs/LICENSE"
install -m 0644 "$ROOT/docs/safety-and-evidence.md" "$BUNDLE_DIR/docs/safety-and-evidence.md"
install -m 0644 "$ROOT/docs/architecture.md" "$BUNDLE_DIR/docs/architecture.md"
install -m 0644 "$ROOT/docs/candidate-identity-v1.md" "$BUNDLE_DIR/docs/candidate-identity-v1.md"
install -m 0644 "$ROOT/docs/source-access-architecture-v1.md" "$BUNDLE_DIR/docs/source-access-architecture-v1.md"
install -m 0644 "$ROOT/docs/desktop-interaction-v2.md" "$BUNDLE_DIR/docs/desktop-interaction-v2.md"
install -m 0644 "$ROOT/docs/gui-workflow-v1.md" "$BUNDLE_DIR/docs/gui-workflow-v1.md"
install -m 0644 "$ROOT/docs/release-process.md" "$BUNDLE_DIR/docs/release-process.md"
install -m 0644 "$ROOT/docs/release-candidate-v0.1.0.md" "$BUNDLE_DIR/docs/release-candidate-v0.1.0.md"
install -m 0644 "$ROOT/docs/project-status.md" "$BUNDLE_DIR/docs/project-status.md"
install -m 0644 "$ROOT/docs/dependency-advisories.md" "$BUNDLE_DIR/docs/dependency-advisories.md"
install -m 0644 "$ROOT/docs/linux-distribution-v1.md" "$BUNDLE_DIR/docs/linux-distribution-v1.md"
install -m 0644 "$ROOT/docs/local-release-evidence-v1.md" "$BUNDLE_DIR/docs/local-release-evidence-v1.md"
install -m 0644 "$ROOT/docs/case-brief-v1.md" "$BUNDLE_DIR/docs/case-brief-v1.md"
install -m 0644 "$ROOT/docs/future-github-launch-v1.md" "$BUNDLE_DIR/docs/future-github-launch-v1.md"
install -m 0644 "$ROOT/docs/release-notes-v0.1.0-draft.md" "$BUNDLE_DIR/docs/release-notes-v0.1.0-draft.md"

cat > "$BUNDLE_DIR/launch-disktrace.sh" <<'EOF'
#!/bin/sh
set -eu
BUNDLE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$BUNDLE_DIR/bin/evidenceforge-desktop" "$@"
EOF
chmod 0755 "$BUNDLE_DIR/launch-disktrace.sh"

cat > "$BUNDLE_DIR/install-disktrace-launcher.sh" <<'EOF'
#!/bin/sh
set -eu
BUNDLE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
APPLICATIONS_DIR=${XDG_DATA_HOME:-"$HOME/.local/share"}/applications
DESKTOP_FILE="$APPLICATIONS_DIR/disktrace-recovery.desktop"
mkdir -p "$APPLICATIONS_DIR"
cat > "$DESKTOP_FILE" <<DESKTOP
[Desktop Entry]
Type=Application
Name=DiskTrace Recovery
Comment=Local-first forensic recovery workspace
Exec="$BUNDLE_DIR/launch-disktrace.sh"
Terminal=false
Categories=Utility;System;
StartupNotify=true
DESKTOP
printf '%s\\n' "installed $DESKTOP_FILE"
EOF
chmod 0755 "$BUNDLE_DIR/install-disktrace-launcher.sh"

cat > "$BUNDLE_DIR/release-manifest.json" <<EOF
{
  "schema_version": 1,
  "product": "DiskTrace",
  "version": "$VERSION",
  "target": "$TARGET",
  "format": "tar.gz",
  "license": "Apache-2.0",
  "source_commit": "$SOURCE_COMMIT",
  "source_state": "clean-committed",
  "supported_build_host": "Linux x86_64",
  "primary_launcher": "launch-disktrace.sh",
  "included_binaries": [
    "bin/evidenceforge-desktop",
    "bin/evidenceforge"
  ],
  "intentional_limits": [
    "Not code signed or notarized",
    "Not an installer or automatic update channel",
    "Not validated for Windows, macOS, or non-x86_64 Linux",
    "Not a public release artifact"
  ]
}
EOF

(
    cd "$BUNDLE_DIR"
    find bin docs -type f -print | LC_ALL=C sort | xargs sha256sum
    sha256sum launch-disktrace.sh install-disktrace-launcher.sh release-manifest.json
) > "$BUNDLE_DIR/SHA256SUMS"

rm -f "$ARCHIVE_PATH" "$CHECKSUM_PATH"
(
    cd "$STAGING_ROOT"
    tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
        -czf "$ARCHIVE_PATH" "$BUNDLE_NAME"
)
(
    cd "$OUTPUT_DIR"
    sha256sum "$ARCHIVE_NAME"
) > "$CHECKSUM_PATH"

printf '%s\n' "created $ARCHIVE_PATH"
printf '%s\n' "created $CHECKSUM_PATH"
