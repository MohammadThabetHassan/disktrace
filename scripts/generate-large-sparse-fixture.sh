#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUTPUT=${1:?usage: generate-large-sparse-fixture.sh <output-image>}
OFFSET_BYTES=${OFFSET_BYTES:-33554432}
TOTAL_BYTES=${TOTAL_BYTES:-67108864}
SEED_IMAGE="$ROOT/fixtures/fat12-deleted-file-v1/source.img"

case "$OFFSET_BYTES:$TOTAL_BYTES" in
    *[!0-9:]*|:*)
        printf '%s\n' 'OFFSET_BYTES and TOTAL_BYTES must be non-negative decimal byte counts' >&2
        exit 1
        ;;
esac

seed_bytes=$(wc -c < "$SEED_IMAGE" | tr -d ' ')
if [ "$OFFSET_BYTES" -gt "$TOTAL_BYTES" ] || [ "$seed_bytes" -gt $((TOTAL_BYTES - OFFSET_BYTES)) ]; then
    printf '%s\n' 'the seed fixture does not fit inside the requested sparse image' >&2
    exit 1
fi

mkdir -p "$(dirname -- "$OUTPUT")"
rm -f "$OUTPUT"
truncate -s "$TOTAL_BYTES" "$OUTPUT"
dd if="$SEED_IMAGE" of="$OUTPUT" bs=1 seek="$OFFSET_BYTES" conv=notrunc status=none

printf '%s\n' "generated deterministic sparse PNG-carving fixture: $OUTPUT"
printf '%s\n' "total_bytes=$TOTAL_BYTES seed_offset=$OFFSET_BYTES seed_bytes=$seed_bytes expected_png_offset=$((OFFSET_BYTES + 4096))"
