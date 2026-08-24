set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fixture_dir="$project_root/fixtures/foundation-session-v1"
source_image="$fixture_dir/source.img"
workspace=$(mktemp -d)
trap 'rm -rf "$workspace"' EXIT
mkdir -p "$workspace/destination"

sh "$project_root/scripts/generate-foundation-fixture.sh"
actual_hash=$(sha256sum "$source_image" | awk '{print $1}')
expected_hash=$(cat "$source_image.sha256")
[ "$actual_hash" = "$expected_hash" ]

cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- inspect "$source_image" > "$workspace/session.json"
grep -q '"read_only": true' "$workspace/session.json"
grep -q "\"sha256\": \"$expected_hash\"" "$workspace/session.json"

if cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- check-destination "$source_image" "$fixture_dir" > "$workspace/unsafe.out" 2> "$workspace/unsafe.err"; then
  exit 1
fi
grep -q 'source image storage location' "$workspace/unsafe.err"

cargo run -q -p ef-cli --manifest-path "$project_root/Cargo.toml" -- check-destination "$source_image" "$workspace/destination" > "$workspace/approved-destination.txt"
grep -q "$workspace/destination" "$workspace/approved-destination.txt"

printf '%s\n' 'foundation verification passed'
