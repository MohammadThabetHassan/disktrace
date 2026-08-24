# Candidate Identity v1 Contract

## Purpose

A recovery candidate ID is an internal evidence handle used by selection, saved sessions, receipts, audit records, and safe export naming. It must identify the same recovered byte range and method when discovery order changes. It is **not** an authenticity claim, a cryptographic signature of the recovered bytes, or a substitute for the source SHA-256/BLAKE3 identity gate.

## Stable ID format

New scans use the following ASCII-only format:

```text
efc1-<method>-<source-offset-hex>-<byte-length-hex>-<blake3-identity-hex>
```

The `efc1` prefix identifies the DiskTrace Candidate Identity v1 encoder. The method, source offset, and byte length are readable fixed-width values. The full BLAKE3 identity digest is computed over a domain-separated, length-prefixed encoding of the recovery method, validation state, file type, evidence name, source offset, byte length, and optional original path. Including all of these immutable `RecoveryCandidate` facts prevents parser-list order from becoming an identifier authority and distinguishes coincident byte ranges that have different recovered metadata.

The candidate ID deliberately does not include source-image hashes. Those hashes belong to the separate recovery-session identity and are rechecked before a saved-session export. This keeps IDs reproducible for a given discovered candidate while the session remains the authority binding that candidate to one source image.

## Recovery lookup

Stable-ID recovery re-discovers the current v1 parser candidates, derives each candidate’s `efc1` ID from immutable facts, selects the matching candidate, and then extracts the same candidate through the existing bounded method-specific recovery path. The re-derived candidate must still match the selected stable candidate after extraction. No byte range is accepted directly from an ID.

> A stable identifier removes parser-order coupling; it does not weaken parser validation, source-identity verification, destination policy, or saved-session candidate equality checks.

## Compatibility

Earlier local manifests use legacy index-addressed IDs such as `png-carve-0000`. They remain accepted by an isolated legacy recovery route so existing local sessions and recorded exports do not become inaccessible. New discovery never emits those IDs. The prefix distinguishes the formats without schema ambiguity, so the current session-manifest schema remains readable while new manifests persist stable IDs.

## Verification

The migration requires deterministic checks that stable IDs are unchanged when candidate vectors are reordered, differ when any identity-bound fact changes, remain unique across the complete supported fixture suite, recover byte-for-byte through each current method, and preserve legacy-ID recovery for existing manifests. Existing source-identity and manifest equality gates remain required for export.
