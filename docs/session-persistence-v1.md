# DiskTrace Session Persistence v1

## Purpose

A recovery session is a **local, portable evidence workspace record**. It preserves the source identity, the candidate catalogue generated from that source, and the recovery exports made through DiskTrace. It never contains recovered file bytes, transmits no data, and does not cause automatic rescans, automatic exports, or cloud synchronization.

The session format is intended to let a user stop and return later while retaining a clear answer to three questions: which image was scanned, whether the currently available image still matches that image, and what files were exported with what hashes.

## Manifest boundary

A persisted session uses a JSON manifest with a versioned schema. The manifest contains the immutable `RecoverySession`, the deterministic candidate catalogue from its completed scan, and an append-only-in-memory list of export records. Each export record holds the candidate identifier, saved artifact path, receipt path, export timestamp, and the complete recovery receipt.

| Field | Role | Safety property |
|---|---|---|
| `schema_version` | Specifies the manifest layout. | Unsupported versions are rejected rather than guessed. |
| `session` | Records source path, byte length, both source hashes, session identifier, policy version, and status. | The original source identity remains immutable. |
| `candidates` | Stores the scan results that the user reviewed. | A reload does not silently replace evidence with a new scan. |
| `exports` | Records completed recovery operations and their receipts. | Exported artifact hashes remain inspectable from the session file. |

The manifest path is explicitly chosen by the user. DiskTrace uses atomic replacement when updating an existing manifest: it writes and synchronizes a sibling temporary file before replacing the target. A failed manifest update must not alter the existing manifest.

## Integrity state

> A loaded candidate catalogue is historical evidence about an earlier scan. It is not permission to export from whatever file now happens to exist at the recorded path.

Every reload performs a fresh read-only inspection of the stored canonical source path. The result is one of the following states.

| State | Meaning | Recovery behavior |
|---|---|---|
| **Verified** | Current byte length, SHA-256, and BLAKE3 all match the manifest. | Recovery from the saved candidate catalogue is allowed. |
| **Changed** | The source is readable but its identity differs from the stored source identity. | Recovery is blocked. The user can scan the changed image as a separate session. |
| **Unavailable** | The stored source cannot be opened or inspected. | Recovery is blocked. The historical catalogue and export history remain readable. |

A successful integrity check does not mutate the saved source identity or session identifier. A changed image must receive a new scan and a new session rather than overwriting the original evidence record.

## Recovery and export behavior

A recovery performed from a loaded, verified manifest uses the original session identifier and source identity when producing a receipt. This keeps every export attributable to the same scanned source. Destination approval, exclusive output creation, artifact synchronization, and receipt hashing remain mandatory.

Only a successfully written recovery export may be appended to the manifest history. The manifest history references the receipt data but does not trust a receipt path as a guarantee that the file still exists; receipt and artifact hashes are historical provenance values captured at export time.

## User interface behavior

The desktop workspace provides explicit actions to save the current session and to open a saved session. It displays the session identifier, source identity summary, persistence location, integrity state, candidate count, and export history. A loaded session with a changed or unavailable source remains inspectable, but its recovery action is disabled with a plain-language explanation.

The command-line interface exposes the same model: users can create a manifest from a scan, inspect or verify a manifest, and recover a candidate only after source verification. Existing one-command scan and recovery commands remain available for straightforward, non-resumable use.

## Out of scope for v1

This version does not implement encrypted manifests, signing keys, a case-management database, a background file watcher, device acquisition, multi-user synchronization, or automatic source relocation. Those features would need separate threat models and interaction designs.
