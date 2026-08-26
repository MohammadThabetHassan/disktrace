# ZIP/Open XML windowed discovery v1

## Decision and scope

DiskTrace may discover ZIP and supported Open XML candidates through bounded, read-only source windows while retaining the existing full-buffer ZIP carver as the compatibility oracle. This is the fifth bounded discovery route after PNG, JPEG, GIF, and PDF. It changes discovery only: filesystem parsing, other carvers, recovery, export, session rederivation, and audit behavior retain their existing compatibility paths.

> A successful windowed ZIP/Open XML candidate means that the source-backed parser reproduced the supported legacy ZIP structural checks and package classification for one bounded byte range. It does not establish original filename, path, completeness, archive safety, decompression success, XML validity, authenticity, legal admissibility, or recovery of every deleted archive.

## Compatibility source of truth

The legacy `ef-carve::carve_zip_archives` behavior is normative for this increment. It accepts a local-file header, retains the existing **64 MiB absolute candidate cap**, finds an end-of-central-directory record, rejects split/empty/sentinel-sized or inconsistent directory records, requires the declared central-directory end to equal the end-record offset, validates every declared central-directory entry, and checks that each referenced local header and entry name agree. It classifies a valid package as `docx`, `xlsx`, or `pptx` only when its entry names include the legacy-required Open XML paths; otherwise it classifies it as `zip`. A malformed candidate is skipped by advancing the legacy search by one byte; a valid candidate suppresses new starts through its end.

The source-window route must emit candidates that are exactly equal to the legacy candidates after stable candidate identity is assigned, including method, evidence name, file type, source offset, byte length, validation, and ordering. Count mismatch, field mismatch, or an extra windowed candidate is a scan failure. The scanner must not silently choose one discovery route over the other.

## Window ownership and reads

| Control | v1 rule |
| --- | --- |
| Primary window length | 1 MiB of source-owned bytes. |
| Recognized signature | `PK\x03\x04`, four bytes. |
| Detection overlap | three bytes after each primary window, used only to recognize a local-file header that begins in the primary window and ends across the boundary. |
| Ownership | A local-file header belongs only to the primary window containing its first byte. Bytes from the overlap never create an additional candidate owner. |
| Candidate validation | One local file handle and checked bounded range reads. The parser retains one 1 MiB read buffer plus fixed Open XML membership flags; it does not retain archive payload bytes or an unbounded entry-name collection. |
| Candidate limit | `min(source length, start offset + 64 MiB)` with source-end and arithmetic safety. |
| Structural parsing | Recreate the legacy local-header signature, end-record, central-directory range, entry count, local-header/name consistency, and `zip`/Open XML classification rules. |
| Suppression and ordering | After acceptance, suppress local-header starts strictly before the accepted end and number candidates in source order. A malformed header does not suppress a later valid header. |
| Cancellation | Check before route work, before primary-window work, inside bounded source reads, and after a completed primary window. Cancellation produces no successful scan result. |

## Required controls

The first implementation must add deterministic controls for all of the following conditions:

1. Exact parity against the committed ZIP/Open XML document fixture candidates.
2. A `PK\x03\x04` local-file header beginning in the final three bytes of a primary window.
3. A malformed boundary local header followed by a valid later candidate.
4. Adjacent valid candidates with stable ordering and legacy-compatible suppression.
5. A truncated candidate at source end and a candidate that exceeds the absolute 64 MiB bound.
6. Central-directory mismatch, invalid local-header/name reference, and Open XML classification refusal controls.
7. Cancellation after a completed primary window.
8. Candidate-cap source-end and integer-overflow semantics.
9. Workflow-level parity failure when a windowed result diverges from the legacy candidate list.

## Explicit non-claims

This work does not make DiskTrace a full streaming scanner. The scanner continues to buffer the complete source for filesystem metadata, the legacy compatibility carvers, AVI/MP4/MOV discovery, recovery, export, and audit rederivation. It does not add ZIP64, split-archive, decompression, payload extraction beyond the existing recovery path, archive repair, XML validation, PDF or document rendering, parser-loop progress, a cancellation-latency guarantee, a general performance benchmark, recovery of fragmented/overwritten data, source writing, external rendering, cloud processing, accounts, telemetry, runtime AI, signing, notarization, or a public release.

## Verification entry points

The implementation belongs in the ordinary full local matrix only after its focused script proves the dedicated controls. Required evidence is formatting, strict relevant linting, the focused ZIP/Open XML window-discovery script, the existing document recovery and desktop contracts, `sh scripts/verify-all.sh`, and all required hosted workflows on the exact pushed commit.
