#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
LINUX_ARCHIVE=${1:?usage: sh scripts/generate-local-release-evidence.sh <linux-archive> <windows-cross-target-archive> [output-path]}
WINDOWS_ARCHIVE=${2:?usage: sh scripts/generate-local-release-evidence.sh <linux-archive> <windows-cross-target-archive> [output-path]}
VERSION=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$ROOT/Cargo.toml" | head -n 1)
OUTPUT_PATH=${3:-"$ROOT/dist/EvidenceForge-$VERSION-local-evidence.json"}

if [ -z "$VERSION" ]; then
    printf '%s\n' 'Unable to determine workspace version from Cargo.toml' >&2
    exit 1
fi
for command in sha256sum stat uname date; do
    if ! command -v "$command" >/dev/null 2>&1; then
        printf '%s\n' "Local release-evidence generation requires: $command" >&2
        exit 1
    fi
done

cd "$ROOT"
sh scripts/verify-linux-bundle.sh "$LINUX_ARCHIVE"
sh scripts/verify-windows-cross-target-bundle.sh "$WINDOWS_ARCHIVE"

linux_name=$(basename "$LINUX_ARCHIVE")
windows_name=$(basename "$WINDOWS_ARCHIVE")
linux_hash=$(sha256sum "$LINUX_ARCHIVE" | awk '{print $1}')
windows_hash=$(sha256sum "$WINDOWS_ARCHIVE" | awk '{print $1}')
linux_size=$(stat -c '%s' "$LINUX_ARCHIVE")
windows_size=$(stat -c '%s' "$WINDOWS_ARCHIVE")
generated_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
host_os=$(uname -s)
host_architecture=$(uname -m)

mkdir -p "$(dirname "$OUTPUT_PATH")"
cat > "$OUTPUT_PATH" <<EOF
{
  "schema_version": 1,
  "product": "EvidenceForge Recovery",
  "version": "$VERSION",
  "record_type": "local_verification_evidence",
  "generated_at_utc": "$generated_at",
  "host": {
    "operating_system": "$host_os",
    "architecture": "$host_architecture"
  },
  "artifacts": [
    {
      "name": "$linux_name",
      "target": "linux-x86_64",
      "format": "tar.gz",
      "sha256": "$linux_hash",
      "byte_size": $linux_size,
      "verification": "linux bundle archive, staged checksum, CLI, and native desktop smoke passed"
    },
    {
      "name": "$windows_name",
      "target": "windows-x86_64",
      "format": "zip",
      "sha256": "$windows_hash",
      "byte_size": $windows_size,
      "verification": "cross-target archive, staged checksum, launcher, Wine CLI, and Wine/Xvfb desktop smoke passed"
    }
  ],
  "commands_executed": [
    "sh scripts/verify-linux-bundle.sh $linux_name",
    "sh scripts/verify-windows-cross-target-bundle.sh $windows_name"
  ],
  "intentional_limits": [
    "Local verification evidence only; not a public release record",
    "Windows artifact is Linux-host cross-target and Wine compatibility evidence, not native Windows release evidence",
    "No Authenticode signing, SmartScreen, native Windows installer, macOS validation, hosted CI, repository governance, tag, or public release evidence"
  ]
}
EOF

printf '%s\n' "created $OUTPUT_PATH"
