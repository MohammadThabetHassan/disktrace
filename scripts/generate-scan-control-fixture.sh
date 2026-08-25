#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
SEED_IMAGE="$ROOT/fixtures/fat12-deleted-file-v1/source.img"
PNG_OFFSET_IN_SEED=4096

usage() {
    printf '%s\n' 'usage: generate-scan-control-fixture.sh <scenario> <output-image>' >&2
    printf '%s\n' '       generate-scan-control-fixture.sh --describe <scenario>' >&2
    exit 1
}

configure_scenario() {
    SCENARIO=$1
    TOTAL_BYTES=0
    SEED_OFFSETS=''
    INVALID_SIGNATURE_INTERVAL=0

    case "$SCENARIO" in
        large-sparse-png-v1)
            TOTAL_BYTES=$((64 * 1024 * 1024))
            SEED_OFFSETS=$((32 * 1024 * 1024))
            ;;
        signature-dense-png-v1)
            TOTAL_BYTES=$((16 * 1024 * 1024))
            SEED_OFFSETS=$((8 * 1024 * 1024))
            INVALID_SIGNATURE_INTERVAL=$((16 * 1024))
            ;;
        signature-dense-refusal-v1)
            TOTAL_BYTES=$((16 * 1024 * 1024))
            INVALID_SIGNATURE_INTERVAL=$((16 * 1024))
            ;;
        multi-candidate-png-v1)
            TOTAL_BYTES=$((32 * 1024 * 1024))
            SEED_OFFSETS="$((4 * 1024 * 1024)) $((16 * 1024 * 1024)) $((24 * 1024 * 1024))"
            ;;
        *)
            printf '%s\n' "unsupported scan-control scenario: $SCENARIO" >&2
            exit 1
            ;;
    esac

    EXPECTED_PNG_OFFSETS=''
    for seed_offset in $SEED_OFFSETS; do
        png_offset=$((seed_offset + PNG_OFFSET_IN_SEED))
        if [ -n "$EXPECTED_PNG_OFFSETS" ]; then
            EXPECTED_PNG_OFFSETS="$EXPECTED_PNG_OFFSETS,$png_offset"
        else
            EXPECTED_PNG_OFFSETS=$png_offset
        fi
    done

    if [ -n "$EXPECTED_PNG_OFFSETS" ]; then
        EXPECTED_PNG_COUNT=$(printf '%s\n' "$EXPECTED_PNG_OFFSETS" | awk -F, '{ print NF }')
    else
        EXPECTED_PNG_COUNT=0
    fi
}

print_description() {
    printf '%s\n' "scenario=$SCENARIO"
    printf '%s\n' "total_bytes=$TOTAL_BYTES"
    printf '%s\n' "expected_png_count=$EXPECTED_PNG_COUNT"
    printf '%s\n' "expected_png_offsets=$EXPECTED_PNG_OFFSETS"
}

if [ "${1:-}" = '--describe' ]; then
    [ "$#" -eq 2 ] || usage
    configure_scenario "$2"
    print_description
    exit 0
fi

[ "$#" -eq 2 ] || usage
configure_scenario "$1"
OUTPUT=$2

case "$OUTPUT" in
    ''|/|.)
        printf '%s\n' 'output image path must name a file.' >&2
        exit 1
        ;;
esac
if [ -d "$OUTPUT" ]; then
    printf '%s\n' 'output image path must not be a directory.' >&2
    exit 1
fi

seed_bytes=$(wc -c < "$SEED_IMAGE" | tr -d ' ')
for seed_offset in $SEED_OFFSETS; do
    if [ "$seed_offset" -gt "$TOTAL_BYTES" ] || [ "$seed_bytes" -gt $((TOTAL_BYTES - seed_offset)) ]; then
        printf '%s\n' "seed image does not fit in $SCENARIO at offset $seed_offset" >&2
        exit 1
    fi
done

mkdir -p "$(dirname -- "$OUTPUT")"
rm -f "$OUTPUT"
truncate -s "$TOTAL_BYTES" "$OUTPUT"

for seed_offset in $SEED_OFFSETS; do
    dd if="$SEED_IMAGE" of="$OUTPUT" bs=1 seek="$seed_offset" conv=notrunc status=none
done

if [ "$INVALID_SIGNATURE_INTERVAL" -gt 0 ]; then
    WORK=$(mktemp -d)
    cleanup() {
        rm -rf "$WORK"
    }
    trap cleanup EXIT HUP INT TERM
    printf '\211PNG\r\n\032\n' > "$WORK/invalid-png-signature.bin"

    signature_offset=0
    while [ "$signature_offset" -lt "$TOTAL_BYTES" ]; do
        inside_seed=0
        for seed_offset in $SEED_OFFSETS; do
            if [ "$signature_offset" -ge "$seed_offset" ] && [ "$signature_offset" -lt $((seed_offset + seed_bytes)) ]; then
                inside_seed=1
                break
            fi
        done
        if [ "$inside_seed" -eq 0 ]; then
            dd if="$WORK/invalid-png-signature.bin" of="$OUTPUT" bs=1 seek="$signature_offset" conv=notrunc status=none
        fi
        signature_offset=$((signature_offset + INVALID_SIGNATURE_INTERVAL))
    done
fi

print_description
printf '%s\n' "output=$OUTPUT"
