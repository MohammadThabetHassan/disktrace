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
| `read_window` | Read a bounded sliding window with explicit overlap for signature detection and structural parsing. | Future file-backed carving scanner. |
| `read_all_compatibility` | Provide the current contiguous bytes only for recovery paths not yet migrated. | Existing `fs::read`, explicitly marked transitional. |

## Migration stages

The first implementation stage is complete for selected previews. `ef-core` now performs complete source identity verification on one local file handle and reads only the selected candidate’s exact range with checked arithmetic, source-length checks, fallible allocation, and cooperative cancellation between 1 MiB reads. `ef-workflow` accepts only a candidate persisted in the completed session; the desktop preview worker passes a manifest snapshot and continues to discard stale generations. This removes full-image preview buffering while preserving bounded preview parsing. Exports intentionally remain on their existing full source-identity verification and candidate-rederivation path, preserving the stricter manifest comparison until that path has its own separately verified migration.

The second stage will introduce a windowed scanning primitive with a fixed, documented window and overlap contract. A parser must either determine a valid bounded length entirely inside available bytes or request a new bounded continuation. Format-specific maximum carve lengths remain mandatory. Discovery migration will begin with the structurally simplest signatures and retain byte-for-byte fixture comparisons.

The third stage will migrate filesystem readers only after their sector, cluster, and record access patterns have a source-backed equivalent and their current fixture suite is extended with sparse and multi-gigabyte-shaped test images. Memory mapping is not assumed: it requires platform and failure-mode evidence before adoption.

## Cancellation and progress

The current compatibility path implements the first cooperative-cancellation stage. A shared atomic signal is checked before source inspection, before opening a source for identity hashing, and between 1 MiB hash and full-image buffering reads. The workflow checks again before and after candidate discovery. A cancelled scan creates no session manifest or candidate catalogue, preserves any prior completed session, and lets the desktop distinguish “stopping current work” from “completed cancellation.”

This is intentionally not full parser-level cancellation. Current filesystem and carving discovery still consume an already-buffered byte slice, so a stop requested while an individual discovery routine runs can be observed only after discovery returns. No partial result is applied. Progress reporting is deferred until source access is windowed; it must then be measured from source bytes examined rather than candidate counts.

## Measured compatibility baseline

The local [Scan performance baseline v1](../local-verification/scan-performance-baseline-v1.md) runs the current CLI scan path 20 times against each committed deterministic fixture and records elapsed time plus candidate-count stability. Its largest committed source is 2.01 MiB. The separate [large sparse scan control v1](../local-verification/scan-performance-large-sparse-v1.md) generates a 64 MiB source with one validated PNG at a fixed offset and records three compatibility-path scans. It proves candidate stability for that controlled source and makes full-buffer scan cost visible at a larger size, but it is not real-device, multi-gigabyte, signature-dense, or fragmented-filesystem evidence. No streaming, memory-mapping, or search-algorithm claim is justified from either corpus alone.

## Required verification

The completed range-reader stage has deterministic controls for exact bytes, zero-length source-end reads, source-end refusal, integer-overflow refusal, changed-length refusal, substituted same-length source refusal, cancellation before source access, and cancellation after the first bounded range chunk. A parity regression compares range reads against the established full-buffer recovery path for every candidate in every committed recovery fixture. The desktop additionally proves that a same-length source substitution is refused even when its cached UI source-status value was previously verified. Windowed discovery still requires its own cancellation-after-window and before/after corpus evidence. Before and after measurements must use identical sparse, signature-dense, and multi-candidate fixtures.
