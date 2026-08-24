set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fixture_dir="$project_root/fixtures/foundation-session-v1"
mkdir -p "$fixture_dir"
printf 'EvidenceForge foundation fixture\nscenario=read-only-session\nrevision=1\n' > "$fixture_dir/source.img"
sha256sum "$fixture_dir/source.img" | awk '{print $1}' > "$fixture_dir/source.img.sha256"
printf '%s\n' '{"fixture_id":"foundation_session_v1","scenario":"read_only_image_import","source":"source.img","expected":"session_manifest_with_read_only_source"}' > "$fixture_dir/manifest.json"
