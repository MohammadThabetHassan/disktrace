#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

cargo test -p ef-workflow windowed_zip_candidate_limit_matches_the_legacy_cap_semantics
cargo test -p ef-workflow windowed_zip_discovery_matches_the_legacy_document_fixture_candidates
cargo test -p ef-workflow scan_accepts_zip_candidates_only_after_windowed_legacy_parity
cargo test -p ef-workflow windowed_zip_discovery_owns_a_signature_straddling_the_primary_boundary
cargo test -p ef-workflow invalid_boundary_zip_does_not_hide_a_later_valid_candidate
cargo test -p ef-workflow windowed_zip_discovery_preserves_adjacent_candidate_ordering
cargo test -p ef-workflow windowed_zip_discovery_refuses_a_truncated_candidate_at_source_end
cargo test -p ef-workflow windowed_zip_discovery_refuses_a_candidate_beyond_the_absolute_cap
cargo test -p ef-workflow windowed_zip_discovery_refuses_a_central_directory_mismatch
cargo test -p ef-workflow windowed_zip_discovery_refuses_a_mismatched_local_header_name
cargo test -p ef-workflow windowed_zip_discovery_preserves_open_xml_classification_boundaries
cargo test -p ef-workflow windowed_zip_discovery_cancels_after_a_completed_primary_window

printf '%s\n' 'windowed ZIP/Open XML discovery verification passed'
