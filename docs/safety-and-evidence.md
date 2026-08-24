# Safety and evidence boundaries

DiskTrace is a **local recovery and evidence-review application**. It reads a user-selected image file, identifies selected recovery candidates using bounded parsers and carvers, and writes an explicitly requested export to a separate approved directory. The tool is designed to make its limits visible rather than hiding uncertainty behind a successful-looking export.

> A recovered candidate is a record of what the implemented method accepted at a specified source byte range. It is not proof that the recovered bytes are complete, original, safe to open, legally admissible, or attributable to a particular person or directory path.

## Operating boundaries

| Boundary | DiskTrace behavior |
|---|---|
| **Source handling** | Reads a local image file. The source is not mounted or opened for writing by the application. |
| **Identity** | Captures source byte length, canonical path, SHA-256, and BLAKE3 at session creation. |
| **Destination safety** | Refuses destinations on source storage, nested destinations, symlinks, and missing directories. Exports use create-new semantics to avoid overwrite. |
| **Sessions** | Saved sessions retain source identity, candidate data, and receipt-backed history locally. A reopened source is rehashed before any recovery export is allowed. |
| **Receipts** | Every successful export receives a JSON receipt tying the artifact hash, candidate source range, method, validation state, destination policy result, and source identity together. |
| **Network behavior** | The application does not include cloud upload, AI telemetry, or remote recovery services. |
| **Preview behavior** | Text is shown only as a bounded excerpt. Binary candidates remain metadata-only rather than being rendered or executed. |

## Required operator practice

Create and preserve a stable source image before running recovery work whenever possible. Keep the original storage untouched, document how the image was produced, and use a separate export destination. Record the tool version, source identity, session manifest, and receipt with any result that may need later review.

Recovered files can contain hostile, incomplete, sensitive, or misleading data. Treat them as untrusted. Do not double-click an export on a production workstation. Use a disposable or isolated environment, avoid macros and embedded active content, and apply organization-specific malware and privacy procedures before opening material with an end-user application.

## Interpretation of validation labels

| Label | Meaning | What it does not mean |
|---|---|---|
| **Recovered and checked** | The supported internal structure required by the carver was present. | The file is semantically complete, safe, trusted, malware-free, or linked to an original path. |
| **Recovered — review recommended** | The metadata method accepted its narrow structural and range checks and exported the listed bytes. | The bytes have not been overwritten, are complete, or remain associated with the displayed name/path. |
| **Likely intact** | A future method may use this label only when its defined integrity checks support it. | Legal proof, authorship, or universal compatibility with another application. |
| **May be incomplete** | The method encountered a documented limitation or partial condition. | The result should be silently repaired or treated as usable without review. |
| **Not recoverable with this source** | The current source or method does not meet the method’s acceptance rules. | The file cannot exist elsewhere or cannot be recovered by a different authorized method. |

## Method-specific caution

Metadata recovery depends on surviving filesystem structures. Deleted FAT entries may retain only a short name and a previously referenced cluster chain. exFAT and contiguous NTFS recovery can require the current allocation bitmap to describe an extent as free; free allocation is an observation about current allocation state, not evidence that the prior bytes were never reused. NTFS resident recovery exports bytes retained inside an accepted fixed-up MFT record. Structural carving works from raw byte patterns and cannot recover original file-system location or establish all context around the bytes it accepts.

Exact acceptance and refusal conditions are versioned in the method contracts under [`docs/`](.). Review those contracts before drawing conclusions from a candidate or modifying the parser.

## Out of scope

DiskTrace does not attempt to bypass encryption, access controls, passwords, or device authentication. It does not repair files, infer unsupported metadata, reconstruct arbitrary fragmented streams, verify legal chain of custody, determine ownership or intent, render arbitrary recovered content, scan for malware, provide incident-response advice, or certify evidentiary admissibility.

The application is not a substitute for a qualified digital-forensics process, legal review, incident-response procedure, malware-analysis environment, or secure evidence-storage practice. When the outcome is consequential, preserve originals, maintain contemporaneous notes, and use the procedures required by the relevant organization or jurisdiction.

## Reporting a safety or security concern

Do not attach real images, recovered private data, authentication material, or active exploit samples to a public report. Follow [SECURITY.md](../SECURITY.md) for private vulnerability reporting expectations. For a parser false positive or recovery-boundary concern, provide a minimized synthetic reproduction when possible and state the method, candidate ID, source offsets, tool version, expected result, and observed result.
