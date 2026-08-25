#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

cargo test -p ef-workflow windowed_jpeg_discovery_matches_the_legacy_fixture_candidates
cargo test -p ef-workflow windowed_jpeg_discovery_owns_a_signature_straddling_the_primary_boundary
cargo test -p ef-workflow invalid_boundary_jpeg_does_not_hide_a_later_valid_candidate
cargo test -p ef-workflow windowed_jpeg_discovery_preserves_adjacent_candidate_ordering
cargo test -p ef-workflow windowed_jpeg_discovery_refuses_a_truncated_candidate_at_source_end
cargo test -p ef-workflow windowed_jpeg_discovery_cancels_after_a_completed_primary_window

printf '%s\n' 'windowed JPEG discovery verification passed'
