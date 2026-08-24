#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUTPUT_DIRECTORY=${1:-"$ROOT/dist/sbom"}

case "$OUTPUT_DIRECTORY" in
    /*) ;;
    *) OUTPUT_DIRECTORY="$ROOT/$OUTPUT_DIRECTORY" ;;
esac

if [ "$OUTPUT_DIRECTORY" = "/" ] || [ "$OUTPUT_DIRECTORY" = "$ROOT" ]; then
    printf '%s\n' 'SBOM output directory must be a dedicated subdirectory.' >&2
    exit 1
fi

cd "$ROOT"
if ! git diff --quiet || ! git diff --cached --quiet; then
    printf '%s\n' 'SBOM generation requires a clean tracked source revision.' >&2
    exit 1
fi
if [ -n "$(git ls-files --others --exclude-standard)" ]; then
    printf '%s\n' 'SBOM generation requires no untracked source files.' >&2
    exit 1
fi
if ! cargo cyclonedx --version >/dev/null 2>&1; then
    printf '%s\n' 'cargo-cyclonedx 0.5.9 is required to generate the SBOM.' >&2
    exit 1
fi

GENERATOR_VERSION=$(cargo cyclonedx --version | awk '{print $2}')
if [ "$GENERATOR_VERSION" != "0.5.9" ]; then
    printf '%s\n' "Expected cargo-cyclonedx 0.5.9, found $GENERATOR_VERSION." >&2
    exit 1
fi

SOURCE_COMMIT=$(git rev-parse HEAD)
SOURCE_DATE_EPOCH=$(git show -s --format=%ct HEAD)
WORK=$(mktemp -d "${TMPDIR:-/tmp}/disktrace-sbom.XXXXXX")
OUTPUT_PARENT=$(dirname "$OUTPUT_DIRECTORY")

cleanup() {
    rm -rf "$WORK"
}
trap cleanup EXIT HUP INT TERM

mkdir -p "$OUTPUT_PARENT"
rm -rf "$OUTPUT_DIRECTORY"
mkdir -p "$OUTPUT_DIRECTORY"

git archive --format=tar HEAD | tar -x -C "$WORK"
(
    cd "$WORK"
    SOURCE_DATE_EPOCH="$SOURCE_DATE_EPOCH" \
        cargo cyclonedx --format json --all --target all --spec-version 1.5 --quiet
)

DOCUMENT_COUNT=0
find "$WORK" -type f -name '*.cdx.json' -print | LC_ALL=C sort | while IFS= read -r source_path; do
    relative_path=${source_path#"$WORK"/}
    destination_path="$OUTPUT_DIRECTORY/$relative_path"
    mkdir -p "$(dirname "$destination_path")"
    cp "$source_path" "$destination_path"
    DOCUMENT_COUNT=$((DOCUMENT_COUNT + 1))
    printf '%s\n' "$DOCUMENT_COUNT" > "$OUTPUT_DIRECTORY/.document-count"
done

if [ ! -f "$OUTPUT_DIRECTORY/.document-count" ]; then
    printf '%s\n' 'cargo-cyclonedx did not produce a CycloneDX JSON document.' >&2
    exit 1
fi
DOCUMENT_COUNT=$(cat "$OUTPUT_DIRECTORY/.document-count")
rm -f "$OUTPUT_DIRECTORY/.document-count"

(
    cd "$OUTPUT_DIRECTORY"
    find . -type f -name '*.cdx.json' -print | LC_ALL=C sort | sed 's|^./||' |
        while IFS= read -r relative_path; do
            sha256sum "$relative_path"
        done > SHA256SUMS
)

cat > "$OUTPUT_DIRECTORY/sbom-provenance.json" <<EOF
{
  "schema_version": 1,
  "source_commit": "$SOURCE_COMMIT",
  "source_date_epoch": $SOURCE_DATE_EPOCH,
  "generator": {
    "name": "cargo-cyclonedx",
    "version": "$GENERATOR_VERSION"
  },
  "format": "CycloneDX 1.5 JSON",
  "target_scope": "all Cargo targets",
  "document_count": $DOCUMENT_COUNT
}
EOF

printf '%s\n' "generated $DOCUMENT_COUNT CycloneDX SBOM documents in $OUTPUT_DIRECTORY"
