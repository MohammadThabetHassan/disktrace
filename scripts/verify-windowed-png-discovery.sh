#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

cargo test -p ef-workflow windowed_png_discovery_matches_the_legacy_fixture_candidates
cargo test -p ef-workflow windowed_png_discovery_owns_a_signature_straddling_the_primary_boundary
cargo test -p ef-workflow invalid_boundary_png_does_not_hide_a_later_valid_candidate
cargo test -p ef-workflow windowed_png_discovery_suppresses_nested_signatures_like_the_legacy_carver
cargo test -p ef-workflow windowed_png_discovery_cancels_after_a_completed_primary_window

printf '%s\n' 'windowed PNG discovery verification passed'
