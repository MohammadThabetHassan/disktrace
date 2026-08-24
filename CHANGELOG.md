# Changelog

All notable changes to DiskTrace are documented here. The workspace is currently local and pre-release: entries below describe verified implementation work, not published versions or public release artifacts.

## Unreleased

### Added

- Local read-only image inspection with SHA-256 and BLAKE3 source identity.
- Immutable recovery sessions, versioned local session manifests, source-integrity verification, and receipt-backed export history.
- Destination policy that rejects source-storage destinations, nested destinations, symlinks, missing directories, and output overwrite.
- Recovery receipts with source identity, candidate byte range, recovery method, validation state, and artifact hashes.
- FAT12 and FAT16 deleted short-name root-directory recovery with retained cluster-chain extraction.
- Bounded exFAT deleted root-file recovery for intact checksummed entry sets with contiguous extents currently marked free by the allocation bitmap.
- Bounded NTFS deleted resident-record recovery after FILE-record update-sequence validation.
- Bounded NTFS deleted non-resident recovery for one uncompressed, non-sparse, unnamed contiguous former extent currently marked free by the allocation bitmap.
- Structural PNG, JPEG, GIF, standard RIFF/AVI, self-contained MP4/MOV, PDF, and ZIP/Open XML carving methods with method-specific refusal conditions.
- Deterministic candidate catalogue filtering, sorting, validation summaries, plain-language explanations, bounded text previews, and metadata-only binary previews.
- Bounded local structure summaries for recovered PNG, JPEG, GIF, AVI, MP4/MOV, PDF, and ZIP/Open XML candidates, including format-specific header or container facts without rendering, opening, decoding, decompressing, or executing recovered content.
- Deterministic GIF/AVI/MP4 synthetic media fixture and end-to-end CLI verifier covering source offsets, method filters, bounded facts, separate-destination enforcement, byte-for-byte export, and malformed-container refusal checks.
- Native cross-platform desktop workspace with guided Recovery Mode, Evidence Mode, background scan cancellation, local pickers, method filtering, source-integrity state, saved-session controls, and export history.
- Deterministic synthetic fixtures and end-to-end verifier scripts for every current recovery method.
- Public project documentation, safety boundaries, contribution guidance, security policy, code of conduct, Apache-2.0 license, and local CI configuration.
- A locally verified Linux x86_64 distribution bundle with staged-file and archive checksums.
- A native Windows x86_64 portable ZIP and Inno Setup installer configuration, checksum verifier, and hosted Windows build workflow. These Windows paths are configured but have not yet received native Windows build, launch, install, uninstall, signing, or hosted-workflow evidence.

### Changed

- The desktop workflow now includes persistent local-only and read-only-source status, a scrollable step-progress rail, a stronger first-run orientation panel, resettable filters, evidence cards with method and validation badges, session metrics, and a refined safe-export detail view.
- The desktop visual system now uses a named graphite, slate, cyan, mineral-green, amber, and coral palette. The palette separates structural surfaces from active focus and makes verification, review, and failure semantics consistent across the recovery workflow.
- The native desktop workflow now includes a discoverable platform-command shortcut reference, adaptive candidate and evidence-detail sizing, state-specific result guidance, a disabled-until-ready scan action, a single first-run primary action location, and keyboard navigation across filtered evidence results. These refinements retain native system dialogs and avoid WebView, telemetry, and framework dependencies.
- Recovery export now opens a native final-review window that identifies the selected candidate, recovery method, validation state, recovered byte count, verified session status, and requested destination. Cancelling this review produces no output; confirmation preserves the existing destination-policy and receipt-backed export path.
- The desktop workspace now follows a quieter casework hierarchy: an inline case-status header, lower-chrome workflow rail, concise first-run start sheet, compact evidence-session strip with progressive records, and selected-result emphasis that avoids repeated dashboard cards and status chips.
- Evidence sessions now expose a read-only **Recheck source** action that refreshes source identity against the saved session without rescanning, exporting, changing the image, or discarding the historical catalogue.
- Desktop catalogue refresh now creates metadata-only presentations and derives bounded text or structure previews only when a user selects a candidate. This avoids repeated source-image reads for every filtered result while retaining the existing selected-evidence preview and export boundaries.
- Selected previews now prepare through a generation-stamped local background worker. The detail pane makes this short read-only state visible, keeps result navigation responsive, discards stale preview work after a new selection, and never opens, renders, executes, or exports a file as part of preview preparation.
- Selected-preview buffering now shares cooperative cancellation with superseded selections, workspace resets, and catalogue refreshes. New preview bytes are withheld when source identity is changed or unavailable, previously derived preview facts are reset to metadata-only on a failed recheck, and a current preview-worker failure is shown explicitly instead of being presented as an unsupported format. Bounded extraction remains non-interruptible after buffering under the current source-access architecture.
- Selected previews now recheck the full saved source identity on one local handle and then read only the persisted candidate’s exact byte range with checked arithmetic, source-end refusal, fallible allocation, and 1 MiB cancellation checks. Exact-range output is parity-tested against the existing recovery path for every candidate in every committed fixture, including a desktop control for a same-length source substitution. Export recovery intentionally retains its stricter full rederivation and manifest comparison path.
- Background scans now share a cooperative cancellation signal with source inspection, source hashing, and full-image buffering. The desktop distinguishes a requested stop from an acknowledged stop, preserves the prior completed catalogue, and applies no partial replacement result. Current in-memory discovery routines remain non-interruptible within an individual parser loop.
- New scans now assign Candidate Identity v1 (`efc1`) handles from immutable recovery facts rather than parser-list position. Stable IDs are re-derived before bounded extraction and persist in new sessions, receipts, audits, and safe export names; legacy index-addressed IDs remain recoverable for existing local manifests.
- Selected evidence now follows an audited reading hierarchy: **At a glance** presents recovery method, validation, byte count, and source offset before bounded preview facts; method limitations follow; the long trace handle is kept in a collapsed record. Separate-destination guidance and the final review boundary remain unchanged.
- A repeatable local scan-baseline harness now records elapsed time and candidate-count stability across the existing deterministic fixtures. Its largest 2.01 MiB source is explicitly recorded as insufficient evidence for a streaming, memory-mapping, or signature-algorithm change.
- Receipt-backed exports can now be audited from Evidence Mode or through `audit-session`. The audit compares persisted receipt JSON and current output SHA-256/BLAKE3 values against the recorded session receipt, reporting verified, changed, unavailable, or inconsistent evidence without changing any file. A deterministic CLI scenario covers both verified output and a deliberate post-export artifact change.
- Completed sessions can now produce a local, payload-free Markdown **case brief** from Evidence Mode or the `case-brief` CLI command. It recomputes source identity and recorded export-audit state, summarizes candidates and their bounded methods, and documents explicit limitations without uploading or publishing data.
- Local future-GitHub launch materials now include an authorization-gated launch checklist and a v0.1.0 draft release narrative. They prepare a later repository launch without initializing Git, creating commits, pushing, tagging, running hosted workflows, or creating a release today.
- A repeatable Linux-host Windows cross-target compatibility smoke now builds the actual Windows x86_64 desktop and CLI binaries and checks them under Wine/X11. This is documented as compatibility evidence only; native Windows packaging, installer, signing, and usability validation remain required.
- The local Windows engineering path now builds and verifies a cross-target portable ZIP with the desktop and CLI binaries, conventional `.cmd` launcher, documentation, staged-file checksums, archive checksum, and packaged Wine/Xvfb smoke. A machine-readable local release-evidence record binds this ZIP and the verified Linux bundle to their executed checks and explicit non-release limitations.
- A vetted project-local desktop-design skill now guides native eframe/egui workflow changes. The selected-result panel independently scrolls and makes missing-destination or source-integrity blocks explicit before it offers recovery.
- Desktop rendering now uses eframe 0.33.3, resolving the browser-launch dependency onto the fixed webbrowser 1.2.4 release line while retaining the established guided desktop workflow.
- The pinned Rust toolchain now includes Clippy, and the local verification matrix enforces `cargo clippy --workspace --all-targets -- -D warnings`, warning-free workspace documentation generation, and `cargo audit`; Linux and Windows hosted workflows mirror the applicable checks.
- Recovery labels now distinguish filesystem metadata evidence, current allocation state, and structurally validated carving results.
- Saved-session recovery rechecks source identity before export and keeps changed/unavailable sessions historical-only.

### Intentional limitations

- No direct device acquisition, physical-drive access, image creation, encryption bypass, password recovery, cloud upload, telemetry, or AI-assisted recovery.
- No filesystem repair, universal or generic fragmented-file reconstruction, path reconstruction, long filename recovery for FAT, recursive directory recovery, arbitrary NTFS runlists, alternate streams, or semantic file validation. GIF carving requires a complete supported block stream and trailer; AVI support excludes RF64/OpenDML extensions; MP4/MOV support requires a self-contained non-fragmented `ftyp`/`moov`/`mdat` layout and does not validate playback, codecs, or sample offsets.
- No Authenticode-signed Windows installer, Windows-native artifact, hosted CI evidence, public issue tracker, maintainer support SLA, or tagged release yet.

## Release process

A public release should be created only after the local verification matrix, dependency/security checks, hosted CI, repository governance, installer/artifact validation, release notes, and maintainer contact routes are complete. See [docs/release-process.md](docs/release-process.md).
