#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

for pattern in \
    'struct Palette' \
    'Palette::FOCUS' \
    'Palette::VERIFIED' \
    'Palette::REVIEW' \
    'Palette::DANGER' \
    'show_shortcuts' \
    'show_recovery_review' \
    'recheck_source_integrity' \
    'Recheck source' \
    'Source remains verified' \
    'Audit exports' \
    'Save case brief' \
    'choose_case_brief_to_save' \
    'save_case_brief' \
    'audit_recorded_exports' \
    'Recorded exports verified' \
    'export_audit_presentation' \
    'begin_recovery_review' \
    'recovery_review_window' \
    'egui::Modal::new' \
    'recovery_review_modal' \
    'Review recovery export' \
    'Confirm recovery and create receipt' \
    'select_result_by_offset' \
    'Use Up/Down to review the filtered results' \
    'Keyboard shortcuts' \
    'Cmd/Ctrl + O (letter)' \
    'workspace_empty_panel' \
    'start_workspace_panel' \
    'casework_step' \
    'detail_width' \
    'The rail continues with filters' \
    'Replace image…' \
    'Local only' \
    'Read-only source' \
    'Recovery workspace' \
    'Recovery workflow' \
    'Case record' \
    'Reset' \
    'Evidence detail' \
    'At a glance' \
    'candidate_at_a_glance' \
    'Method notes' \
    'Candidate record' \
    'Choose a separate destination' \
    'Recovery remains blocked' \
    'evidence_detail_scroll' \
    'action_guidance_panel' \
    'Recover selected file safely' \
    'PreviewKind::StructureSummary' \
    'Bounded structure summary' \
    'ScrollArea::vertical' \
    'recovery_review_requires_verified_source_and_does_not_export_until_confirmed' \
    'result_navigation_stays_within_filtered_presentations' \
    'source_recheck_refreshes_integrity_without_discarding_candidates' \
    'export_audit_reports_verified_and_changed_recorded_outputs' \
    'saves_a_case_brief_from_the_current_local_session' \
    'background_scan_applies_document_candidates' \
    'PreviewWorker' \
    'start_selected_preview' \
    'poll_preview_worker' \
    'selected_preview_is_loading' \
    'selected_preview_error' \
    'read_session_candidate_range' \
    'selected_preview_rechecks_source_identity_before_reading_a_range' \
    'Bounded local preview unavailable' \
    'rechecking the saved source identity' \
    'Preparing bounded local preview' \
    'select_result' \
    'load_media_fixture' \
    'Media' \
    'MethodFilter::Gif' \
    'MethodFilter::Avi' \
    'MethodFilter::Mp4' \
    'METHOD_GIF' \
    'METHOD_AVI' \
    'METHOD_MP4' \
    'SignatureCarvingGif' \
    'SignatureCarvingAvi' \
    'SignatureCarvingMp4' \
    'background_scan_applies_media_candidates_with_structure_summaries' \
    'workflow_state_tracks_image_and_scan_progress' \
    'ScanWorkerEvent' \
    'scan_image_with_cancellation' \
    'STOPPING SCAN' \
    'Stopping local scan' \
    'Scan stopped' \
    'cancelling_a_pending_scan_preserves_the_previous_catalogue' \
    'cancelling_selected_preview_signals_the_worker_and_discards_its_result' \
    'unverified_source_withholds_selected_preview_bytes' \
    'selected_preview_rechecks_source_identity_before_reading_a_range' \
    'current_preview_worker_failure_remains_visible'; do
    grep -q "$pattern" crates/ef-desktop/src/main.rs
done

if grep -Eq 'Color32::from_rgb\([0-9]{1,3},' crates/ef-desktop/src/main.rs; then
    printf '%s\n' 'desktop palette verification failed: found an ad hoc decimal RGB literal' >&2
    exit 1
fi

grep -q 'native confirmation window' docs/gui-workflow-v1.md
grep -q 'quiet casework' docs/gui-workflow-v1.md
grep -q 'Up`/`Down` to compare' docs/gui-workflow-v1.md
grep -q 'Audit exports' docs/gui-workflow-v1.md
grep -q 'Save case brief' docs/gui-workflow-v1.md
grep -q 'self-contained MP4/MOV' docs/gui-workflow-v1.md
grep -q 'Preparing bounded local preview' docs/gui-workflow-v1.md
grep -q 'Bounded local preview unavailable' docs/gui-workflow-v1.md
grep -q 'cooperatively stops a superseded preview' docs/gui-workflow-v1.md
grep -q 'exact local byte range' docs/gui-workflow-v1.md
grep -q 'quick or full format' docs/gui-workflow-v1.md
grep -q 'Stopping scan' docs/gui-workflow-v1.md
grep -q 'partial scan results' docs/gui-workflow-v1.md
grep -q 'At a glance' docs/gui-workflow-v1.md
grep -q 'collapsed candidate record' docs/gui-workflow-v1.md
grep -q 'PreviewFact' crates/ef-catalogue/src/lib.rs
grep -q 'candidate_preview_structure' crates/ef-catalogue/src/lib.rs
grep -q 'gif_preview_facts' crates/ef-catalogue/src/lib.rs
grep -q 'avi_preview_facts' crates/ef-catalogue/src/lib.rs
grep -q 'mp4_preview_facts' crates/ef-catalogue/src/lib.rs
cargo test -p ef-desktop recovery_review_requires_verified_source_and_does_not_export_until_confirmed
cargo test -p ef-desktop result_navigation_stays_within_filtered_presentations
cargo test -p ef-desktop source_recheck_refreshes_integrity_without_discarding_candidates
cargo test -p ef-desktop export_audit_reports_verified_and_changed_recorded_outputs
cargo test -p ef-desktop saves_a_case_brief_from_the_current_local_session
cargo test -p ef-desktop background_scan_applies_document_candidates
cargo test -p ef-desktop background_scan_applies_media_candidates_with_structure_summaries
cargo test -p ef-desktop workflow_state_tracks_image_and_scan_progress
cargo test -p ef-desktop cancelling_a_pending_scan_preserves_the_previous_catalogue
cargo test -p ef-desktop cancelling_selected_preview_signals_the_worker_and_discards_its_result
cargo test -p ef-desktop unverified_source_withholds_selected_preview_bytes
cargo test -p ef-desktop selected_preview_rechecks_source_identity_before_reading_a_range
cargo test -p ef-desktop current_preview_worker_failure_remains_visible
cargo test -p ef-workflow cancelled_preview_recovery_refuses_to_access_a_source
cargo test -p ef-workflow verified_session_ranges_match_full_recovery_for_every_fixture_candidate

printf '%s\n' 'desktop UI contract verification passed'
