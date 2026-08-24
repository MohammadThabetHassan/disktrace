#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
FIXTURE="$ROOT/fixtures/media-carving-multimethod-v1"
SOURCE="$FIXTURE/source.img"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
DESTINATION="$WORK/destination"
mkdir -p "$DESTINATION"

python3 "$ROOT/scripts/generate-media-fixture.py"
EXPECTED_SHA256=$(sha256sum "$SOURCE" | awk '{print $1}')
grep -q "$EXPECTED_SHA256" "$FIXTURE/manifest.json"

cargo run -q -p ef-cli --manifest-path "$ROOT/Cargo.toml" -- help > "$WORK/help.txt" 2>&1
grep -q 'gif|avi|mp4|mov' "$WORK/help.txt"

cargo run -q -p ef-cli --manifest-path "$ROOT/Cargo.toml" -- scan "$SOURCE" > "$WORK/scan.json"
gif_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$WORK/scan.json" --method signature_carving_gif --source-offset 1024)
avi_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$WORK/scan.json" --method signature_carving_avi --source-offset 8192)
mp4_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$WORK/scan.json" --method signature_carving_mp4 --source-offset 16384)
printf '%s\n' "$gif_id" | grep -q '^efc1-signature_carving_gif-'
printf '%s\n' "$avi_id" | grep -q '^efc1-signature_carving_avi-'
printf '%s\n' "$mp4_id" | grep -q '^efc1-signature_carving_mp4-'
grep -q '"source_offset": 1024' "$WORK/scan.json"
grep -q '"method": "signature_carving_gif"' "$WORK/scan.json"
grep -q '"id": "efc1-' "$WORK/scan.json"
grep -q '"source_offset": 8192' "$WORK/scan.json"
grep -q '"method": "signature_carving_avi"' "$WORK/scan.json"
grep -q '"id": "efc1-' "$WORK/scan.json"
grep -q '"source_offset": 16384' "$WORK/scan.json"
grep -q '"method": "signature_carving_mp4"' "$WORK/scan.json"

cargo run -q -p ef-cli --manifest-path "$ROOT/Cargo.toml" -- catalogue "$SOURCE" > "$WORK/catalogue.json"
grep -q '"total_candidates": 3' "$WORK/catalogue.json"
grep -q '"carved_candidates": 3' "$WORK/catalogue.json"
grep -q '"kind": "structure_summary"' "$WORK/catalogue.json"
grep -q '"label": "Logical screen"' "$WORK/catalogue.json"
grep -q '"label": "Required lists"' "$WORK/catalogue.json"
grep -q '"label": "Top-level boxes"' "$WORK/catalogue.json"

cargo run -q -p ef-cli --manifest-path "$ROOT/Cargo.toml" -- catalogue "$SOURCE" --method gif --validation content_validated > "$WORK/gif-catalogue.json"
grep -q '"total_candidates": 1' "$WORK/gif-catalogue.json"
grep -q "$gif_id" "$WORK/gif-catalogue.json"
if grep -q "$avi_id" "$WORK/gif-catalogue.json"; then
    printf '%s\n' 'GIF method filter returned an AVI candidate' >&2
    exit 1
fi

if cargo run -q -p ef-cli --manifest-path "$ROOT/Cargo.toml" -- recover "$SOURCE" "$gif_id" "$FIXTURE" > "$WORK/unsafe.out" 2> "$WORK/unsafe.err"; then
    exit 1
fi
grep -q 'source image storage location' "$WORK/unsafe.err"

cargo run -q -p ef-cli --manifest-path "$ROOT/Cargo.toml" -- recover "$SOURCE" "$gif_id" "$DESTINATION" > "$WORK/gif-receipt.json"
cargo run -q -p ef-cli --manifest-path "$ROOT/Cargo.toml" -- recover "$SOURCE" "$avi_id" "$DESTINATION" > "$WORK/avi-receipt.json"
cargo run -q -p ef-cli --manifest-path "$ROOT/Cargo.toml" -- recover "$SOURCE" "$mp4_id" "$DESTINATION" > "$WORK/mp4-receipt.json"
cmp "$FIXTURE/expected-carved.gif" "$DESTINATION/$gif_id.gif"
cmp "$FIXTURE/expected-carved.avi" "$DESTINATION/$avi_id.avi"
cmp "$FIXTURE/expected-carved.mp4" "$DESTINATION/$mp4_id.mp4"
grep -q '"recovery_method": "signature_carving_gif"' "$WORK/gif-receipt.json"
grep -q '"recovery_method": "signature_carving_avi"' "$WORK/avi-receipt.json"
grep -q '"recovery_method": "signature_carving_mp4"' "$WORK/mp4-receipt.json"

cargo test -q -p ef-carve rejects_fragmented_or_incomplete_mp4_content
cargo test -q -p ef-carve rejects_avi_without_required_media_list

printf '%s\n' 'media carving verification passed'
