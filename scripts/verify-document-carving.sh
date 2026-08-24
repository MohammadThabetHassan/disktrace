#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

python3 "$ROOT/scripts/generate-document-fixture.py" >/dev/null
SOURCE="$ROOT/fixtures/document-carving-multimethod-v1/source.img"
PDF_EXPECTED="$ROOT/fixtures/document-carving-multimethod-v1/expected-carved.pdf"
DOCX_EXPECTED="$ROOT/fixtures/document-carving-multimethod-v1/expected-carved.docx"
DIRECT_DESTINATION="$WORK/direct-destination"
SESSION_DESTINATION="$WORK/session-destination"
SESSION_MANIFEST="$WORK/evidenceforge-session.json"
mkdir -p "$DIRECT_DESTINATION" "$SESSION_DESTINATION"

EXPECTED_SHA256=$(sha256sum "$SOURCE" | awk '{print $1}')
grep -q "$EXPECTED_SHA256" "$ROOT/fixtures/document-carving-multimethod-v1/manifest.json"

cargo run -q -p ef-cli -- scan "$SOURCE" > "$WORK/scan.json"
pdf_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$WORK/scan.json" --method signature_carving_pdf --source-offset 1024)
docx_id=$(python3 "$ROOT/scripts/select-candidate-id.py" "$WORK/scan.json" --method signature_carving_zip_office --file-type docx --source-offset 16384)
printf '%s\n' "$pdf_id" | grep -q '^efc1-signature_carving_pdf-'
printf '%s\n' "$docx_id" | grep -q '^efc1-signature_carving_zip_office-'
grep -q '"source_offset": 1024' "$WORK/scan.json"
grep -q '"id": "efc1-' "$WORK/scan.json"
grep -q '"file_type": "docx"' "$WORK/scan.json"
grep -q '"source_offset": 16384' "$WORK/scan.json"

cargo run -q -p ef-cli -- catalogue "$SOURCE" --method pdf > "$WORK/pdf-catalogue.json"
grep -q "$pdf_id" "$WORK/pdf-catalogue.json"
grep -q '"kind": "structure_summary"' "$WORK/pdf-catalogue.json"
grep -q '"label": "Format"' "$WORK/pdf-catalogue.json"
grep -q '"label": "Final EOF marker"' "$WORK/pdf-catalogue.json"
if grep -q "$docx_id" "$WORK/pdf-catalogue.json"; then
    printf '%s\n' 'PDF method filter returned a ZIP/Open XML candidate' >&2
    exit 1
fi

cargo run -q -p ef-cli -- catalogue "$SOURCE" --method office > "$WORK/office-catalogue.json"
grep -q "$docx_id" "$WORK/office-catalogue.json"
grep -q 'ZIP-based container' "$WORK/office-catalogue.json"
grep -q '"kind": "structure_summary"' "$WORK/office-catalogue.json"
grep -q '"label": "Central-directory entries"' "$WORK/office-catalogue.json"
grep -q '"label": "Sample package entries"' "$WORK/office-catalogue.json"

cargo run -q -p ef-cli -- recover "$SOURCE" "$pdf_id" "$DIRECT_DESTINATION" > "$WORK/pdf-export.json"
cargo run -q -p ef-cli -- recover "$SOURCE" "$docx_id" "$DIRECT_DESTINATION" > "$WORK/docx-export.json"
cmp "$PDF_EXPECTED" "$DIRECT_DESTINATION/$pdf_id.pdf"
cmp "$DOCX_EXPECTED" "$DIRECT_DESTINATION/$docx_id.docx"
grep -q '"recovery_method": "signature_carving_pdf"' "$WORK/pdf-export.json"
grep -q '"source_range_start": 1024' "$WORK/pdf-export.json"
grep -q '"recovery_method": "signature_carving_zip_office"' "$WORK/docx-export.json"
grep -q '"source_range_start": 16384' "$WORK/docx-export.json"

cargo run -q -p ef-cli -- save-session "$SOURCE" "$SESSION_MANIFEST" > "$WORK/session.json"
cargo run -q -p ef-cli -- session-status "$SESSION_MANIFEST" > "$WORK/session-status.json"
grep -q '"state": "verified"' "$WORK/session-status.json"
cargo run -q -p ef-cli -- recover-session "$SESSION_MANIFEST" "$docx_id" "$SESSION_DESTINATION" > "$WORK/session-export.json"
cmp "$DOCX_EXPECTED" "$SESSION_DESTINATION/$docx_id.docx"
grep -q "$docx_id" "$SESSION_MANIFEST"

cargo test -q -p ef-carve rejects_pdf_without_a_consistent_startxref_pointer
cargo test -q -p ef-carve rejects_zip_with_a_mismatched_central_directory_local_offset

printf '%s\n' 'document carving verification passed'
