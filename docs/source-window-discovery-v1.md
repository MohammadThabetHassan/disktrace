# Source-Window Discovery Contract v1

## Purpose and scope

This contract defines the first bounded-discovery increment for DiskTrace: **PNG structural carving from a local source file in fixed sequential windows**. It exists to reduce the amount of source content retained for the PNG discovery pass while preserving the existing candidate identity and recovery semantics.

The increment is deliberately narrow. FAT, exFAT, NTFS, JPEG, GIF, AVI, MP4/MOV, PDF, ZIP/Open XML, full-source recovery, and receipt-backed export remain on their existing compatibility paths. A successful PNG candidate remains a bounded structural observation, not proof of original filename, path, completeness, authenticity, malware safety, or evidentiary admissibility.

> This contract permits DiskTrace to describe **PNG discovery** as windowed only after its parity and boundary controls pass. It does not permit a claim that the complete scan or every recovery method is streaming.

## Source and identity boundary

`scan_image_with_cancellation` first creates `ImageSource`, which canonicalizes the local path, records length, and calculates SHA-256 and BLAKE3 before discovery begins. The PNG window reader opens the canonical local path once, checks its observed length before each bounded range read, and uses the expected source length for every range calculation.

The window reader must use checked `u64` arithmetic, fallible allocation, exact reads, and cooperative cancellation. It may reuse the core range-reader discipline for one open file handle; it must not rehash the entire source independently for every window.

This is stronger than trusting a cached GUI integrity flag but is not a complete TOCTOU solution. A same-length content mutation after the initial identity calculation and before or during window reads cannot be represented as atomically prevented without a platform snapshot or stronger handle semantics. The UI and public documentation must retain that limit.

## Window geometry and ownership

| Constant | v1 value | Rule |
| --- | ---: | --- |
| Primary window length | 1 MiB | Each sequential primary range owns candidate starts in `[primary_start, primary_end)`. |
| Signature overlap | 7 bytes | The next range includes the trailing seven bytes required to detect the eight-byte PNG signature beginning at the end of a primary range. |
| Range read chunk | Existing core 1 MiB read loop | Cancellation is checked before, between, and after chunks. |
| Candidate parser input | Incremental local-file reads | PNG structure is parsed from the signature forward without materializing the candidate payload. |

A signature is **owned only by the primary range containing its first byte**. The overlap exists solely to detect an eight-byte signature beginning near a primary boundary. It does not create a second ownership range. Sequential processing and primary-range ownership prevent duplicate candidates.

## PNG parser contract

The windowed parser begins at an owned signature and reads the PNG signature, then sequential chunk headers and payload extents from the same open file handle. It must accept exactly the structural conditions accepted by the legacy PNG parser:

1. the eight-byte PNG signature is present;
2. the first chunk is `IHDR` with exactly 13 payload bytes;
3. every chunk end is derived with checked arithmetic from its declared `u32` payload length and 12-byte chunk framing;
4. every candidate extent remains at or below the recorded source length;
5. `IEND` is accepted only with zero payload length; and
6. the returned candidate length is the byte count through the `IEND` CRC.

The v1 parser does not add CRC validation, decompression, pixel validation, original-path inference, or a universal PNG size promise. It must not read or execute recovered content as a preview side effect.

## Search and duplicate-suppression rules

The scanner processes primary windows in ascending source-offset order. It searches every possible signature start in the primary ownership range plus the seven-byte overlap. For every valid owned candidate, it records the absolute source offset and returned byte length, then advances the PNG search suppression boundary to the candidate end.

This preserves legacy `carve_pngs` behavior: a valid outer candidate suppresses subsequent signature starts inside its accepted byte range. An invalid signature advances one byte and cannot suppress later starts. A valid candidate extending beyond its owner’s primary window remains owned by its starting window; later windows skip starts before the recorded suppression boundary.

Candidate naming remains globally ordered by absolute source offset. Conversion to `RecoveryCandidate` uses the existing stable Candidate Identity v1 calculation, so the method, file type, evidence name, source offset, length, validation state, and candidate ID must equal the legacy route.

## Cancellation and progress semantics

The reader checks cancellation before opening/reading, between core read chunks, before each primary window, and after completing each primary window. A cancellation result must never be converted into a partial successful scan. Completed candidate output is discarded by the existing scan cancellation behavior unless the entire scan reaches `ScanCompleted`.

v1 progress may report only completed primary windows and the recorded source length. It must not describe parser-level progress or the recoverability of the remaining source until those behaviors are separately implemented and tested.

## Compatibility routing

The first production integration may use the windowed PNG reader while the scan still buffers the source for filesystem metadata and other carving methods. The legacy full-buffer PNG carver remains a test oracle and export re-derivation reference. This means the initial routing improves the PNG discovery path without claiming that total scan memory is bounded.

Full-source candidate recovery and separate-destination export continue to rediscover and compare candidates through the existing compatibility route. Selected previews continue to use their identity-bound exact-range reader.

## Required verification before routing

| Control | Required result |
| --- | --- |
| Existing PNG fixtures | Windowed PNG candidates and stable IDs equal legacy `carve_pngs`/workflow candidates. |
| Boundary signature | A valid PNG whose signature begins within the final seven bytes of a primary range is found exactly once. |
| Cross-window candidate extent | A PNG starting in one primary window and ending in a later window is accepted once with the legacy byte length. |
| Invalid boundary signature | A truncated or malformed cross-boundary PNG is refused and does not suppress a later valid candidate. |
| Duplicate suppression | A valid outer PNG prevents discovery of a nested signature inside its accepted range, matching the legacy route. |
| Source end and arithmetic | Range overflow, source-end short reads, and candidate-end overflow return structured errors or refusal rather than truncating. |
| Cancellation | Cancellation before access and after a completed bounded read/window returns `Cancelled` with no successful result. |
| File parity | Every PNG candidate from committed fixtures matches legacy recovery bytes and candidate metadata. |
| Controlled scale | The sparse PNG control records candidate count and method route; it is not treated as a real-device benchmark. |

## Explicit non-claims

This design does not claim complete streaming discovery, a complete source-mutation TOCTOU solution, parser-level cancellation, universal performance throughput, fragmented-file reconstruction, or support for malformed/overwritten/encrypted/TRIM-affected data. Any expansion must update this contract, add method-specific controls, and retain a compatibility oracle until independently verified.
