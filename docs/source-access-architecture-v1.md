# Source-access architecture v1

## Purpose

DiskTrace currently reads a selected recovery image into a contiguous in-memory byte slice before filesystem parsing, structural carving, candidate rederivation, or export. This preserves a simple, auditable parser model, but it limits practical work on large images and can repeat I/O when a candidate needs to be rederived. This document defines the safe migration path to bounded, read-only source access.

## Current safety properties

The existing parser and carver APIs accept `&[u8]`. Their structural validators deliberately use bounded byte lengths and reject malformed or open-ended structures. The source identity is recomputed before session export, and export recovery compares the rederived candidate to the manifest candidate. Any replacement source-access layer must retain these properties exactly.

> A performance improvement must never silently broaden a carver’s acceptance rule, bypass source-identity checks, or change the byte range bound into an inferred or unbounded range.

## Target abstraction

The initial abstraction will distinguish **metadata inspection**, **bounded byte-range reading**, and **full-buffer compatibility**.

| Operation | Contract | First implementation |
| --- | --- | --- |
| `inspect` | Obtain path, length, and cryptographic source identity without mutating the source. | Existing `ImageSource::inspect`. |
| `read_range` | Recheck the recorded local source identity, then read an exact validated `(offset, length)` range into memory, rejecting overflow, source-end violations, substituted same-length sources, changed lengths, and cancellation without partial output. | **Implemented** for selected previews through one local file handle, 1 MiB bounded reads, and fallible allocation. |
| `read_window` | Read a bounded sliding window with explicit overlap for signature detection and structural parsing. | **Implemented for PNG discovery** with 1 MiB primary windows, seven-byte signature overlap, primary-window ownership, and legacy parity enforcement. |
| `read_all_compatibility` | Provide the current contiguous bytes only for recovery paths not yet migrated. | Existing `fs::read`, explicitly marked transitional. |

## Migration stages

The first implementation stage is complete for selected previews. `ef-core` now performs complete source identity verification on one local file handle and reads only the selected candidate’s exact range with checked arithmetic, source-length checks, fallible allocation, and cooperative cancellation between 1 MiB reads. `ef-workflow` accepts only a candidate persisted in the completed session; the desktop preview worker passes a manifest snapshot and continues to discard stale generations. This removes full-image preview buffering while preserving bounded preview parsing. Exports intentionally remain on their existing full source-identity verification and candidate-rederivation path, preserving the stricter manifest comparison until that path has its own separately verified migration.

The second stage has begun with PNG discovery. The scanner opens the recorded canonical source once, verifies source length for every exact range, owns candidate starts by the primary 1 MiB window, and reads a seven-byte overlap only to detect the eight-byte signature straddling the next boundary. The PNG structural parser reads only its signature and chunk headers from the same handle; it returns a bounded length through `IEND` without retaining candidate payload bytes. Candidate conversion is required to equal the legacy full-buffer PNG candidates, including evidence names, source ranges, validation, and Candidate Identity v1. A divergence fails the scan rather than silently selecting one route. The full source remains buffered for filesystem metadata and every other carving method, so this is an initial bounded discovery route—not a total-scan memory claim. The complete v1 contract is maintained in [Source-window discovery v1](source-window-discovery-v1.md).

The third stage will migrate filesystem readers only after their sector, cluster, and record access patterns have a source-backed equivalent and their current fixture suite is extended with sparse and multi-gigabyte-shaped test images. Memory mapping is not assumed: it requires platform and failure-mode evidence before adoption.

## Cancellation and progress

The current compatibility path implements cooperative cancellation at declared boundaries. A shared atomic signal is checked before source inspection, before opening a source for identity hashing, and between 1 MiB hash and full-image buffering reads. The workflow then checks after every completed legacy discovery method stage: FAT12, FAT16, exFAT, NTFS, PNG, JPEG, GIF, AVI, MP4/MOV, PDF, and ZIP/Open XML. It also checks before and after the overall candidate-discovery route. A cancelled scan creates no session manifest or candidate catalogue, preserves any prior completed session, and lets the desktop distinguish “stopping current work” from “completed cancellation.”

This is intentionally not full parser-level cancellation. PNG window discovery checks cancellation before primary-window work, inside the bounded core range reader, and after each completed primary window; cancellation produces no successful scan result. Filesystem parsing and all non-PNG carving still consume an already-buffered byte slice, and a stop requested while one individual routine runs is observed only after that method stage returns. No partial result is applied. Progress reporting remains deferred until the desktop has a separately tested source-bytes-examined presentation; no parser-level or whole-scan progress claim is made. The exact checkpoint and latency boundary is maintained in [Legacy discovery cancellation v1](legacy-discovery-cancellation-v1.md).

## Measured compatibility baseline

The committed [synthetic performance-control corpus v1](performance-control-corpus-v1.md) defines sparse, signature-dense acceptance, signature-dense refusal, and multi-candidate PNG scenarios. The ordinary matrix proves exact candidate-count and source-offset stability for each generated source; a separate local harness records elapsed time only after those assertions pass. The corpus makes full-buffer scan behavior visible across controlled byte patterns, but it is not real-device, multi-gigabyte, fragmented-filesystem, cache-state, or search-algorithm evidence. No streaming, memory-mapping, or universal throughput claim is justified from this corpus.

## Required verification

The completed range-reader stage has deterministic controls for exact bytes, zero-length source-end reads, source-end refusal, integer-overflow refusal, changed-length refusal, substituted same-length source refusal, cancellation before source access, and cancellation after the first bounded range chunk. A parity regression compares range reads against the established full-buffer recovery path for every candidate in every committed recovery fixture. The desktop additionally proves that a same-length source substitution is refused even when its cached UI source-status value was previously verified.

The completed PNG window-discovery slice has dedicated controls for a committed-fixture parity match, a signature straddling the primary-window boundary, a malformed boundary signature followed by a valid later candidate, nested-signature suppression, and cancellation after a completed primary window. It retains the legacy full-buffer carver as a compatibility oracle. The completed synthetic corpus adds sparse, signature-dense acceptance/refusal, and multi-candidate controls; it remains a bounded compatibility corpus rather than sufficient evidence for broad performance claims or further source-access migration.
