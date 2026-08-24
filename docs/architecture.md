# Architecture

DiskTrace is a Rust workspace that keeps recovery logic independent from its command-line and desktop interfaces. The design centers on a local read-only source, explicit candidate provenance, deterministic recovery behavior, and destination safety.

## System flow

```text
Local image file
      │
      ▼
ImageSource inspection ──► canonical path + byte length + SHA-256 + BLAKE3
      │
      ▼
Shared discovery workflow
      ├── FAT12 / FAT16 metadata parser
      ├── exFAT metadata parser
      ├── NTFS record parser
      └── PNG / JPEG / GIF / AVI / MP4-MOV / PDF / ZIP-Open XML carvers
      │
      ▼
RecoveryCandidate catalogue
      │
      ├── CLI catalogue, filters, and export
      └── Desktop Recovery Mode and Evidence Mode
      │
      ▼
Destination policy ──► approved separate directory
      │
      ▼
Create-new export + receipt + local session history
```

## Workspace crates

| Crate | Responsibility | Boundary |
|---|---|---|
| `ef-core` | Source identity, recovery-session state, candidate vocabulary, hashing primitives. | Does not parse filesystems or write recovered exports. |
| `ef-fat` | Bounded FAT12, FAT16, exFAT, and NTFS parsing plus metadata-based extraction. | Refuses unsupported structures instead of repairing them. |
| `ef-carve` | Bounded structural PNG, JPEG, GIF, standard AVI, self-contained MP4/MOV, PDF, and ZIP/Open XML discovery and extraction. | Does not decode, decompress, render, repair, execute, or reconstruct fragmented candidate content. |
| `ef-catalogue` | Deterministic candidate filtering, sorting, method summaries, explanations, and bounded previews. | Exposes validation limits in user-facing text. |
| `ef-policy` | Destination-storage, nested-path, symlink, and missing-directory checks. | Does not decide whether a candidate is semantically valid. |
| `ef-report` | Receipt serialization and artifact hashing. | Does not rediscover or modify a candidate. |
| `ef-workflow` | Shared scan, recovery, session, integrity, and receipt orchestration. | Used identically by the CLI and desktop application. |
| `ef-cli` | Scriptable local interface. | Delegates recovery decisions to `ef-workflow`. |
| `ef-desktop` | Native `eframe`/`egui` workspace with background scanning and local file pickers. | Delegates discovery and export to `ef-workflow`. |

## Core data model

| Entity | Invariant |
|---|---|
| `ImageSource` | Represents a local source with canonical path, byte length, SHA-256, and BLAKE3 identity. |
| `RecoverySession` | Represents one immutable scan context. A recovery export must match its source identity. |
| `RecoveryCandidate` | Carries a stable ID, evidence name, file type, source offset, length, method, validation state, and optional original path. |
| `SessionManifest` | Versioned local persistence for a completed session, candidate catalogue, source-integrity result, and recorded exports. |
| `RecoveryReceipt` | Records the session identity, candidate range, recovery method, validation state, destination policy result, and recovered artifact hashes. |

## Source-integrity and session behavior

A scan creates a `RecoverySession` from the current `ImageSource`. Saving a session serializes the completed candidate catalogue and export history using a versioned JSON manifest. Opening a session runs a new `ImageSource::inspect` operation and compares byte length, SHA-256, and BLAKE3 to the recorded identity.

| Source state | Catalogue access | Export behavior |
|---|---|---|
| **Verified** | Available | Allowed through the same destination-policy and create-new export path. |
| **Changed** | Historical candidate list remains available | Blocked. The changed source must be scanned as a new session. |
| **Unavailable** | Historical candidate list remains available | Blocked. |

## Recovery and export behavior

The shared workflow rediscovers a candidate by stable ID from the current image after verifying session identity. This avoids treating a serialised candidate list as authority to extract arbitrary ranges. The destination policy runs before output creation. The export uses `create_new`, syncs the recovered bytes, creates a receipt, and records successful exports in an active saved session.

The source byte range is evidence metadata, not a promise that a filesystem-derived name or path survives. The exact parser contract for each method lives under [`docs/`](.).

## Desktop concurrency model

The desktop application runs scan work away from the UI loop and stamps each requested scan with a generation value. Each worker also receives a shared cooperative cancellation signal. The source-identity hash and full-image buffering paths observe that signal between 1 MiB reads; a cancellation produces no new session or candidate catalogue and preserves any prior completed catalogue. The interface distinguishes a requested stop from an acknowledged stop. If a worker had already completed when cancellation was requested, its result is discarded rather than applied. Discovery still parses already-buffered bytes and is not yet interruptible inside individual parser loops.

Selected previews use a separate generation-stamped worker. It rechecks the completed manifest’s full source identity on one local file handle and then reads only the selected candidate’s validated range in bounded chunks; same-length substitutions fail before preview facts are applied. The worker’s stale-result discard and cancellation signal remain independent of the scan worker. Candidate filtering consumes the shared candidate list; export requests retain the established full-buffer rederivation and manifest comparison path so session identity and export history remain coherent.

## Candidate identity

New scans assign Candidate Identity v1 (`efc1`) handles from immutable `RecoveryCandidate` facts instead of parser-list position. Stable-ID recovery re-derives the candidate from the current image and sends it through the same bounded extraction path; the saved-session equality check remains authoritative before export. Existing local index-addressed handles remain supported by an isolated compatibility route. The complete format and verification contract is defined in [Candidate identity v1](candidate-identity-v1.md).

## Source-access evolution

The current discovery paths operate on bounded in-memory compatibility bytes so existing filesystem and structural parsers remain deterministic. The first migration stage is complete for selected previews: it uses verified exact byte-range extraction rather than full-image preview buffering. DiskTrace is still migrating toward read-only bounded source windows before attempting streaming discovery. This migration must preserve source-identity verification, parser acceptance limits, candidate comparisons, and byte-for-byte fixture results; the staged contract is defined in [Source-access architecture v1](source-access-architecture-v1.md).

## Trust boundaries

The source image and every recovered byte range are untrusted input. Filesystem metadata, file names, attribute values, directory records, and carved format structures must remain bounded before use. The desktop does not render arbitrary binary content. Network communication is outside the application’s recovery workflow.

For operational limits and handling recommendations, see [Safety and evidence boundaries](safety-and-evidence.md).
