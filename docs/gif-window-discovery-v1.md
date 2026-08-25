# GIF windowed discovery v1

## Decision and scope

DiskTrace may discover GIF candidates through bounded, read-only source windows while retaining the existing full-buffer GIF carver as the compatibility oracle. This is the third bounded discovery route after PNG and JPEG. It changes discovery only: filesystem parsing, other carvers, recovery, export, session rederivation, and audit behavior retain their existing compatibility paths.

> A successful windowed GIF candidate means that the source-backed parser reproduced the supported legacy GIF structural checks for one bounded byte range. It does not establish original filename, path, completeness, authenticity, safe content, animation playback, legal admissibility, or recovery of every deleted GIF.

## Compatibility source of truth

The legacy `ef-carve::carve_gifs` behavior is normative for this increment. It accepts `GIF87a` or `GIF89a`, retains the existing **64 MiB absolute candidate cap**, validates the logical-screen/global-color-table layout, requires at least one image block, requires a valid LZW minimum code size, parses supported extension/image sub-block sequences, and terminates only at a trailer after an image. A malformed candidate is skipped by advancing the legacy search by one byte; a valid candidate suppresses new starts through its end.

The source-window route must emit candidates that are exactly equal to the legacy candidates after stable candidate identity is assigned, including method, evidence name, source offset, byte length, validation, and ordering. Count mismatch, field mismatch, or an extra windowed candidate is a scan failure. The scanner must not silently choose one discovery route over the other.

## Window ownership and reads

| Control | v1 rule |
| --- | --- |
| Primary window length | 1 MiB of source-owned bytes. |
| Recognized signatures | `GIF87a` and `GIF89a`, each six bytes. |
| Detection overlap | five bytes after each primary window, used only to recognize a header that begins in the primary window and ends across the boundary. |
| Ownership | A header belongs only to the primary window containing its first byte. Bytes from the overlap never create an additional candidate owner. |
| Candidate validation | Read from one local file handle through checked bounded ranges; candidate parsing retains no payload-sized in-memory copy. |
| Candidate limit | `min(source length, start offset + 64 MiB)` with source-end and arithmetic safety. |
| Structural parsing | Recreate the legacy header, logical-screen, global/local color-table, extension, image, LZW-minimum-code-size, sub-block, image-seen, and trailer rules. |
| Suppression and ordering | After acceptance, suppress header starts strictly before the accepted end and number candidates in source order. A malformed header does not suppress a later valid header. |
| Cancellation | Check before route work, before primary-window work, inside bounded source reads, and after a completed primary window. Cancellation produces no successful scan result. |

## Required controls

The first implementation must add deterministic controls for all of the following conditions:

1. Exact parity against committed GIF/media fixture candidates.
2. A `GIF87a` or `GIF89a` header beginning in the final five bytes of a primary window.
3. A malformed boundary header followed by a valid later candidate.
4. Adjacent valid candidates with stable ordering and legacy-compatible suppression.
5. A truncated candidate at source end and a candidate that exceeds the absolute 64 MiB bound.
6. Cancellation after a completed primary window.
7. Candidate-cap source-end and integer-overflow semantics.
8. Workflow-level parity failure when a windowed result diverges from the legacy candidate list.

## Explicit non-claims

This work does not make DiskTrace a full streaming scanner. The scanner continues to buffer the complete source for filesystem metadata, the legacy compatibility carvers, recovery, export, and audit rederivation. It does not add parser-loop progress, a cancellation-latency guarantee, a general performance benchmark, GIF decoding, playback validation, recovery of fragmented/overwritten data, source writing, external rendering, cloud processing, accounts, telemetry, runtime AI, signing, notarization, or a public release.

## Verification entry points

The implementation belongs in the ordinary full local matrix only after its focused script proves the dedicated controls. Required evidence is formatting, strict relevant linting, the focused GIF window-discovery script, the existing media recovery and desktop contracts, `sh scripts/verify-all.sh`, and all required hosted workflows on the exact pushed commit.
