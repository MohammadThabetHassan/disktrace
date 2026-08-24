set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fixture_dir="$project_root/fixtures/fat16-jpeg-multimethod-v1"
source_image="$fixture_dir/source.img"
workspace=$(mktemp -d)
trap 'rm -rf "$workspace"' EXIT
mkdir -p "$workspace/destination"

python3 "$project_root/scripts/generate-fat16-jpeg-fixture.py"
source_hash=$(sha256sum "$source_image" | awk '{print $1}')
manifest_hash=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["source"]["sha256"])' "$fixture_dir/manifest.json")
[ "$source_hash" = "$manifest_hash" ]

cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- scan "$source_image" > "$workspace/scan.json"
fat16_id=$(python3 "$project_root/scripts/select-candidate-id.py" "$workspace/scan.json" --method fat16_deleted_root_metadata --source-offset 10240)
jpeg_id=$(python3 "$project_root/scripts/select-candidate-id.py" "$workspace/scan.json" --method signature_carving_jpeg --source-offset 11264)
printf '%s\n' "$fat16_id" | grep -q '^efc1-fat16_deleted_root_metadata-'
printf '%s\n' "$jpeg_id" | grep -q '^efc1-signature_carving_jpeg-'
grep -q '"evidence_name": "?ECOVER.TXT"' "$workspace/scan.json"
grep -q '"method": "fat16_deleted_root_metadata"' "$workspace/scan.json"
grep -q '"id": "efc1-' "$workspace/scan.json"
grep -q '"evidence_name": "carved-jpeg-0000.jpg"' "$workspace/scan.json"
grep -q '"method": "signature_carving_jpeg"' "$workspace/scan.json"
grep -q '"validation": "content_validated"' "$workspace/scan.json"

cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- catalogue "$source_image" > "$workspace/catalogue.json"
grep -q '"total_candidates": 2' "$workspace/catalogue.json"
grep -q '"metadata_candidates": 1' "$workspace/catalogue.json"
grep -q '"carved_candidates": 1' "$workspace/catalogue.json"
grep -q '"method_label": "Recovered from deleted FAT16 metadata"' "$workspace/catalogue.json"
grep -q '"method_label": "Found by JPEG signature carving"' "$workspace/catalogue.json"
grep -q '"kind": "text_excerpt"' "$workspace/catalogue.json"
grep -q '"kind": "structure_summary"' "$workspace/catalogue.json"
grep -q '"label": "Dimensions"' "$workspace/catalogue.json"
grep -q '"label": "Sample precision"' "$workspace/catalogue.json"

cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- catalogue "$source_image" --search JPEG --method jpeg --validation content_validated > "$workspace/catalogue-jpeg.json"
grep -q '"total_candidates": 1' "$workspace/catalogue-jpeg.json"
grep -q "$jpeg_id" "$workspace/catalogue-jpeg.json"
if grep -q "$fat16_id" "$workspace/catalogue-jpeg.json"; then
  exit 1
fi

if cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- recover "$source_image" "$fat16_id" "$fixture_dir" > "$workspace/unsafe.out" 2> "$workspace/unsafe.err"; then
  exit 1
fi
grep -q 'source image storage location' "$workspace/unsafe.err"

cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- recover "$source_image" "$fat16_id" "$workspace/destination" > "$workspace/fat16-receipt.json"
cmp "$fixture_dir/expected-recovered.txt" "$workspace/destination/$fat16_id.txt"
grep -q '"source_range_start": 10240' "$workspace/fat16-receipt.json"
grep -q '"recovery_method": "fat16_deleted_root_metadata"' "$workspace/fat16-receipt.json"
grep -q '"validation": "recovered_unvalidated"' "$workspace/fat16-receipt.json"

cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- recover "$source_image" "$jpeg_id" "$workspace/destination" > "$workspace/jpeg-receipt.json"
cmp "$fixture_dir/expected-carved.jpg" "$workspace/destination/$jpeg_id.jpg"
grep -q '"source_range_start": 11264' "$workspace/jpeg-receipt.json"
grep -q '"recovery_method": "signature_carving_jpeg"' "$workspace/jpeg-receipt.json"
grep -q '"validation": "content_validated"' "$workspace/jpeg-receipt.json"

if cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- recover "$source_image" "$jpeg_id" "$workspace/destination" > "$workspace/overwrite.out" 2> "$workspace/overwrite.err"; then
  exit 1
fi
grep -q 'File exists' "$workspace/overwrite.err"

printf '%s\n' 'FAT16 and JPEG recovery verification passed'
