# JPEG windowed discovery contract v1

## Scope

This contract defines a proposed bounded, read-only **JPEG candidate-discovery** route. It is intentionally narrower than a streaming scanner, a new recovery method, or a replacement for full-buffer rederivation. The route may be enabled only when its complete candidate list is exactly equal to the established legacy `carve_jpegs` result for the same inspected source.

> A successful windowed JPEG candidate is a legacy-parity structural observation. It does not prove the original filename, path, completeness, authenticity, malware safety, evidentiary admissibility, or recovery of fragmented, overwritten, encrypted, TRIM-affected, controller-discarded, or otherwise unsupported content.

## Source and window geometry

| Item | Contract |
|---|---|
| Source | One local file handle opened from the inspected canonical path. The source remains read-only. |
| Primary range | Fixed 1 MiB primary ranges in ascending offset order. |
| Signature overlap | Exactly one byte after each non-final primary range, because JPEG SOI is `FF D8`. |
| Ownership | A signature belongs only to the primary range containing its first `FF` byte. The overlap completes an SOI beginning at the final primary byte; it never starts a candidate independently. |
| Candidate ordering | Windows are processed in ascending source-offset order. Accepted ranges suppress later SOIs whose first byte lies inside the accepted range, matching legacy `carve_jpegs` search advancement. |
| Cancellation | The route checks before each primary range, through bounded file reads, and after each completed primary range. Cancellation returns no successful scan result. |

## File-backed structural validation

The candidate parser must reproduce `parse_jpeg_length` byte-for-byte in behavior, with source-backed reads rather than a full candidate buffer. It validates SOI, marker-fill runs, standalone markers, segment lengths, frame-marker presence before SOS, and entropy-coded data termination at EOI. A malformed start is refused and scanning resumes at its next byte, exactly as the legacy carver does.

The parser limit is the smaller of the source end and `candidate_start + 128 MiB`; the limit is absolute, checked for overflow, and is not the primary-window end. Segment metadata may be read in exact small ranges. Entropy data must use bounded sequential buffers with retained `FF` seam state, so `FF 00`, restart markers, EOI, malformed marker pairs, and a final lone `FF` have the same behavior across a read seam as in the legacy byte slice.

## Parity and recovery boundary

The compatibility route begins with the existing full source buffer because filesystem metadata, non-PNG/JPEG carving, recovery, exports, and audit rederivation retain their established full-buffer paths. The file-windowed JPEG list is compared candidate-for-candidate to the stable legacy JPEG list, including evidence name, source offset, byte length, validation, recovery method, and Candidate Identity v1. A differing count, order, or candidate fails the scan rather than choosing a preferred route.

Recovery and export continue to derive JPEG bytes from the legacy full buffer. The bounded route does not create a second recovery path, alter candidate acceptance, relax the 128 MiB cap, add content decoding, or change receipt/session behavior.

## Required controls

| Control | Required result |
|---|---|
| Committed-fixture parity | Windowed JPEG candidates equal legacy JPEG candidates on the committed JPEG fixture. |
| Signature boundary | A valid SOI beginning at the final byte of a primary range is accepted once, with exact legacy parity. |
| Malformed boundary start | A malformed boundary SOI is refused and does not hide a later valid candidate. |
| Adjacent candidates | Two separated valid JPEGs preserve candidate count, ordering, names, offsets, lengths, and stable identities. |
| Cancellation | A request observed after a completed primary range returns `Cancelled`; a pre-signalled request accesses no source. |
| Cap/source-end refusals | Truncated candidate ranges and parser limits remain refused under the exact legacy 128 MiB semantics. |

## Explicit non-claims

This contract does **not** establish full-streaming or whole-scan memory boundedness, parser-level cancellation, byte-accurate progress, a cancellation latency bound within one JPEG parser invocation, general performance improvement, real-device throughput, source-mutation prevention, a complete TOCTOU solution, complete JPEG decoding, CRC/decompression validation, or broader recovery capability. PNG remains the only implemented windowed-discovery method until this JPEG route and all required controls are separately implemented and verified.
