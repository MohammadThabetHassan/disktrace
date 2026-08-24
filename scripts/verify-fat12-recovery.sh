set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fixture_dir="$project_root/fixtures/fat12-deleted-file-v1"
source_image="$fixture_dir/source.img"
workspace=$(mktemp -d)
trap 'rm -rf "$workspace"' EXIT
mkdir -p "$workspace/destination"

python3 "$project_root/scripts/generate-fat12-fixture.py"
source_hash=$(sha256sum "$source_image" | awk '{print $1}')
manifest_hash=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["source"]["sha256"])' "$fixture_dir/manifest.json")
[ "$source_hash" = "$manifest_hash" ]

cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- scan "$source_image" > "$workspace/scan.json"
fat12_id=$(python3 "$project_root/scripts/select-candidate-id.py" "$workspace/scan.json" --method fat12_deleted_root_metadata --source-offset 1536)
png_id=$(python3 "$project_root/scripts/select-candidate-id.py" "$workspace/scan.json" --method signature_carving_png --source-offset 4096)
printf '%s\n' "$fat12_id" | grep -q '^efc1-fat12_deleted_root_metadata-'
printf '%s\n' "$png_id" | grep -q '^efc1-signature_carving_png-'
grep -q '"evidence_name": "?ELETED.TXT"' "$workspace/scan.json"
grep -q '"method": "fat12_deleted_root_metadata"' "$workspace/scan.json"
grep -q '"id": "efc1-' "$workspace/scan.json"
grep -q '"evidence_name": "carved-png-0000.png"' "$workspace/scan.json"
grep -q '"method": "signature_carving_png"' "$workspace/scan.json"
grep -q '"validation": "content_validated"' "$workspace/scan.json"

cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- catalogue "$source_image" > "$workspace/catalogue.json"
grep -q '"total_candidates": 2' "$workspace/catalogue.json"
grep -q '"metadata_candidates": 1' "$workspace/catalogue.json"
grep -q '"carved_candidates": 1' "$workspace/catalogue.json"
grep -q '"method_label": "Recovered from deleted FAT12 metadata"' "$workspace/catalogue.json"
grep -q '"method_label": "Found by PNG signature carving"' "$workspace/catalogue.json"
grep -q '"kind": "text_excerpt"' "$workspace/catalogue.json"
grep -q '"kind": "structure_summary"' "$workspace/catalogue.json"
grep -q '"label": "Dimensions"' "$workspace/catalogue.json"
grep -q '"label": "Color type"' "$workspace/catalogue.json"

cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- catalogue "$source_image" --search PNG --method png --validation content_validated > "$workspace/catalogue-png.json"
grep -q '"total_candidates": 1' "$workspace/catalogue-png.json"
grep -q "$png_id" "$workspace/catalogue-png.json"
if grep -q "$fat12_id" "$workspace/catalogue-png.json"; then
  exit 1
fi

if cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- catalogue "$source_image" --method unknown > "$workspace/invalid-filter.out" 2> "$workspace/invalid-filter.err"; then
  exit 1
fi
grep -q 'unsupported recovery method filter' "$workspace/invalid-filter.err"

if cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- recover "$source_image" "$fat12_id" "$fixture_dir" > "$workspace/unsafe.out" 2> "$workspace/unsafe.err"; then
  exit 1
fi
grep -q 'source image storage location' "$workspace/unsafe.err"

cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- recover "$source_image" "$fat12_id" "$workspace/destination" > "$workspace/fat12-receipt.json"
cmp "$fixture_dir/expected-recovered.txt" "$workspace/destination/$fat12_id.txt"
grep -q '"source_range_start": 1536' "$workspace/fat12-receipt.json"
grep -q '"recovery_method": "fat12_deleted_root_metadata"' "$workspace/fat12-receipt.json"
grep -q '"validation": "recovered_unvalidated"' "$workspace/fat12-receipt.json"

cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- recover "$source_image" "$png_id" "$workspace/destination" > "$workspace/png-receipt.json"
cmp "$fixture_dir/expected-carved.png" "$workspace/destination/$png_id.png"
grep -q '"source_range_start": 4096' "$workspace/png-receipt.json"
grep -q '"recovery_method": "signature_carving_png"' "$workspace/png-receipt.json"
grep -q '"validation": "content_validated"' "$workspace/png-receipt.json"

if cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- recover "$source_image" "$fat12_id" "$workspace/destination" > "$workspace/overwrite.out" 2> "$workspace/overwrite.err"; then
  exit 1
fi
grep -q 'File exists' "$workspace/overwrite.err"

printf '%s\n' 'catalogue and multi-method recovery verification passed'
