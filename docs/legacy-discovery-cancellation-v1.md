# Legacy discovery cancellation contract v1

## Purpose

DiskTrace scans run in a local background worker and can be cancelled cooperatively. This contract defines the current cancellation checkpoints around the existing full-buffer discovery path without representing them as complete parser-level cancellation or a progress guarantee.

> Cancellation stops future work at the next declared checkpoint. It does not retroactively interrupt a parser or carver that is already executing inside one legacy method stage.

## Checkpoint model

| Scan segment | Cancellation behavior | Explicit limit |
|---|---|---|
| Before source inspection | A pre-signalled request returns `Cancelled` before source access or identity creation. | It does not prove that an operator can interrupt an already-started operating-system call. |
| Source inspection and full-buffer read | The source-access layer checks the cancellation flag before work and between 1 MiB buffered reads. | A single read call and identity-hash work already in progress are not pre-empted mid-operation. |
| Legacy method discovery | The workflow checks cancellation after each completed method stage: FAT12, FAT16, exFAT, NTFS, PNG, JPEG, GIF, AVI, MP4/MOV, PDF, and ZIP/Open XML. | The individual parser or carver loop remains non-pre-emptive until its method stage returns. |
| Windowed PNG discovery | The bounded PNG path also checks before each primary window, through cancellable range reads, and after each completed primary window. | It does not make non-PNG discovery windowed or establish complete parser-level cancellation. |
| Candidate publication | A cancelled scan returns `Cancelled`; the desktop worker does not apply a new catalogue and preserves any previous completed catalogue. | This does not establish a complete transaction or time-of-check/time-of-use solution. |

## Candidate and safety invariants

A disabled cancellation flag must preserve the legacy candidate list and its order exactly. The cancellation checkpoints do not change parser acceptance rules, source ranges, candidate identities, recovery methods, source-read-only behavior, export destination policy, receipt behavior, or session persistence.

The deterministic workflow controls prove two bounded facts: first, a disabled flag produces legacy discovery parity across all eleven completed method stages; second, a request observed after the first completed stage stops before the next stage begins. Existing controls separately prove pre-signalled scan refusal, read-loop cancellation, PNG primary-window cancellation, desktop-worker catalogue preservation, and preview cancellation.

## Explicit non-claims

This increment does **not** provide an itemized progress percentage, byte-accurate work estimate, parser-loop cancellation, cancellation latency bound within a legacy method, whole-scan streaming, memory-mapped discovery, complete hostile-input resilience, or a solved time-of-check/time-of-use boundary. The scan still identifies, hashes, and buffers the full source before legacy filesystem and non-PNG carving discovery.

Future progress reporting or parser-level cancellation must define a stage model, residual uninterruptible segments, UI behavior, deterministic tests, and a source-access contract before any public claim is made.
