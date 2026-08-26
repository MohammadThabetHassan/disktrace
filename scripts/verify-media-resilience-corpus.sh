#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

cargo test -p ef-carve avi_resilience_corpus_refuses_malformed_declarations_without_a_panic
cargo test -p ef-carve mp4_resilience_corpus_refuses_malformed_declarations_without_a_panic
cargo test -p ef-carve malformed_media_prefixes_do_not_suppress_later_valid_candidates
cargo test -p ef-carve adjacent_media_candidates_preserve_source_order_and_evidence_names

printf '%s\n' 'AVI and MP4/MOV resilience corpus verification passed'
