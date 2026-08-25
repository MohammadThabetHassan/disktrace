# DiskTrace

**Forensic data recovery, without guesswork.**

DiskTrace is a local-first, cross-platform desktop application for reviewing and exporting **supported** deleted or lost-file candidates from disk-image files. It is designed for people who need a guided recovery workflow and for practitioners who need explicit provenance, structural validation labels, source-integrity checks, safe export behavior, and machine-readable receipts.

> DiskTrace reads the selected source image without writing to it. It does not upload source images, recovered bytes, session manifests, telemetry, or runtime recovery data. A recovery candidate is evidence to review, not a promise that an original file is complete, authentic, safe, or legally admissible.

## Why DiskTrace

A recovery tool should be useful without making claims it cannot prove. DiskTrace records how each candidate was found, where its bytes originate, which bounded checks passed, and where the method stops. Exports are allowed only to an approved separate destination and receive a receipt that binds source identity, candidate facts, and artifact hashes. The safety model and current scope are defined in the [safety and evidence contract](docs/safety-and-evidence.md) and [architecture guide](docs/architecture.md).

| Workflow | Outcome |
| --- | --- |
| **Recovery Mode** | A guided native desktop workflow to select a local image, scan in the background, filter results, review bounded previews, and export a selected result to a separate destination. |
| **Evidence Mode** | Candidate method labels, source offsets, validation state, recovery limitations, session identity, source-integrity status, and receipt-backed export history. |
| **Local sessions** | A versioned JSON manifest records the completed catalogue and exports. Before recovery, a reopened session verifies path, byte length, SHA-256, and BLAKE3. |
| **Command-line workflows** | Optional inspection, scanning, filtered catalogues, destination checks, session handling, auditing, and repeatable selected-candidate recovery. |

## Supported recovery methods

DiskTrace prefers narrow, explainable acceptance rules over optimistic repair. A listed candidate means that its stated structural checks passed; it does **not** prove that every original byte, filename, folder, or filesystem state survived deletion.

| Method | Current acceptance boundary | Important limitation |
| --- | --- | --- |
| Deleted FAT12 root metadata | Deleted short-name root entry and readable retained FAT12 chain. | No long filename, directory reconstruction, or damaged-chain repair. |
| Deleted FAT16 root metadata | Deleted short-name root entry and readable retained FAT16 chain. | No long filename, directory reconstruction, or damaged-chain repair. |
| Deleted exFAT root metadata | Valid deleted root entry set, contiguous former extent, and current free allocation bitmap. | A free allocation bit does not prove the former bytes were not overwritten. |
| Deleted NTFS resident record | Valid deleted fixed-up FILE record with accepted resident name and data attributes. | Only bytes retained inside the FILE record are recovered. |
| Deleted NTFS contiguous metadata | Valid deleted fixed-up FILE record with one uncompressed, non-sparse, unnamed contiguous non-resident extent whose bitmap bits are currently free. | Fragmented, sparse, compressed, encrypted, partial, named, and ambiguous streams are refused. |
| PNG structural carving | PNG signature, required IHDR, bounded IEND, and supported structure checks. | No original filename, folder, or full filesystem provenance. |
| JPEG structural carving | JPEG signature, frame marker, scan marker, and end marker. | No continuity guarantee beyond the accepted byte range. |
| GIF structural carving | `GIF87a`/`GIF89a` header, bounded logical-screen and image/extension blocks, and a final trailer. | No animation decoding or recovery of truncated, fragmented, or overwritten streams. |
| Standard AVI structural carving | RIFF `AVI ` form, declared container boundary, aligned chunks/lists, and required `hdrl`/`movi` lists. | RF64, OpenDML extensions, codec validation, and playback verification are out of scope. |
| Self-contained MP4/MOV carving | Recognized `ftyp` brand, finite 32-bit boxes, internally bounded `moov` with movie header/track, and `mdat`. | Fragmented media, zero/extended-size boxes, playback, codecs, sample offsets, and semantic completeness are not supported. |
| PDF structural carving | Header, traditional cross-reference pointer/table, and bounded end marker. | Cross-reference streams, object repair, and semantic-completeness proof are out of scope. |
| ZIP / Open XML carving | ZIP local and central-directory consistency with an exact end record; Open XML classification from required package entries. | ZIP64, split archives, decompression, XML schema validation, repair, and malware assessment are out of scope. |

## Safety boundary

DiskTrace is a **recovery aid**, not a guarantee of evidentiary completeness or legal admissibility. It does not repair source images, alter source storage, mount images read-write, bypass encryption, defeat access controls, acquire arbitrary live devices, scan recovered files for malware, or make legal conclusions.

Use a copy of the relevant storage or a verified image whenever possible. Export only to a separate destination. Treat every recovered file as potentially incomplete, stale, overwritten, hostile, or sensitive, and review it in an isolated environment before opening it with an application that interprets active content.

A quick or full format can remove filesystem metadata while leaving some raw content in place. DiskTrace can search surviving metadata and supported raw structures, so it may recover some files when no later writes occurred. It cannot guarantee every original file, filename, folder, fragment, encrypted item, overwritten sector, or SSD block affected by TRIM or controller-level garbage collection. Read the [safety and evidence contract](docs/safety-and-evidence.md) before relying on recovered material.

## Quick start

### Prerequisites

Install a Rust toolchain compatible with Rust 2021 edition. The local verification matrix is tested with Rust `1.97.1`. The native desktop smoke test uses `xvfb-run` on Linux; ordinary desktop use does not require it.

Launch the desktop application for the guided recovery workflow:

```sh
cargo run -p ef-desktop
```

The application starts with a three-step guide: choose a local image, scan and review evidence details, then recover to a separate existing folder. It also includes local-session controls and a synthetic guided demo that does not require a real image.

The source is always read from a local file path. The application rejects destinations on source storage, nested destinations, symlinks, and missing directories.

### Optional command-line workflows

The CLI supports repeatable evidence workflows but is not required for ordinary GUI recovery:

```sh
cargo run -p ef-cli -- inspect /path/to/image.img
cargo run -p ef-cli -- scan /path/to/image.img
cargo run -p ef-cli -- catalogue /path/to/image.img --method ntfs-contiguous
# Review the method and byte range, then copy the efc1 candidate ID from scan output.
cargo run -p ef-cli -- recover /path/to/image.img efc1-<candidate-id-from-scan> /separate/output-directory
```

## Sessions, previews, and exports

A saved session stores local source paths, dual source hashes, completed candidate details, source-integrity status, and receipt-backed export history. It never stores recovered payload bytes. If a source becomes unavailable or its length, SHA-256, or BLAKE3 changes, the historical session remains readable but recovery is blocked until the changed image is scanned as a new session.

Selected previews recheck the full saved source identity on one local handle and then read only the persisted candidate’s exact byte range. PNG, JPEG, and GIF candidate discovery additionally use method-specific fixed source windows with mandatory legacy parity; filesystem metadata, legacy compatibility discovery, other carvers, recovery, and exports retain their full-buffer compatibility paths. Exports intentionally retain a stricter full-source re-derivation and manifest-comparison path. The [source-access architecture](docs/source-access-architecture-v1.md), [JPEG windowed-discovery contract](docs/jpeg-window-discovery-v1.md), and [GIF windowed-discovery contract](docs/gif-window-discovery-v1.md) describe this staged design and its remaining time-of-check/time-of-use boundary.

```sh
cargo run -p ef-cli -- save-session /path/to/image.img /path/to/session.disktrace.json
cargo run -p ef-cli -- session-status /path/to/session.disktrace.json
cargo run -p ef-cli -- audit-session /path/to/session.disktrace.json
cargo run -p ef-cli -- case-brief /path/to/session.disktrace.json /path/to/case-brief.md
cargo run -p ef-cli -- recover-session /path/to/session.disktrace.json efc1-<candidate-id-from-session> /separate/output-directory
```

## Verification

Run the complete deterministic local verification matrix from the workspace root:

```sh
sh scripts/verify-all.sh
```

The command checks formatting, warning-free Clippy output, workspace documentation generation, locked-dependency advisory policy, unit tests, deterministic filesystem and carving fixtures, direct and saved-session recovery, receipt-backed export auditing, source-range preview contracts, bounded PNG/JPEG/GIF discovery parity controls, synthetic sparse/signature-dense/refusal/multi-candidate scan controls, builds, and a headless native desktop smoke launch on Linux when `xvfb-run` is available.

Every fixture is synthetic and versioned with known expected bytes and source offsets; none represents a real user image. The [synthetic performance-control corpus](docs/performance-control-corpus-v1.md) is a regression aid, not a real-device benchmark. Current local evidence and intentional limits are summarized in the [public project status report](docs/project-status.md).

## Distribution status

A Linux x86_64 bundle and a Windows x86_64 cross-target compatibility bundle can be built locally using the scripts in this repository. The Linux bundle has local native smoke evidence. The native hosted Windows workflow additionally verifies the portable bundle, a disposable silent installer install/uninstall path, and a retained SBOM review artifact. Hosted macOS 14 ARM64 validation builds and checks an unsigned review binary. Those results are bounded CI evidence only; they are not a macOS package, Intel-macOS evidence, signing/notarization, SmartScreen, manual accessibility acceptance, a tagged production release, or a support SLA.

See the [Linux distribution contract](docs/linux-distribution-v1.md), [Windows distribution contract](docs/windows-distribution-v1.md), [macOS validation contract](docs/macos-validation-v1.md), and [project status report](docs/project-status.md) before sharing any build.

## Project structure

| Path | Purpose |
| --- | --- |
| [`crates/ef-core`](crates/ef-core) | Source identity, session model, candidate types, and recovery-method vocabulary. |
| [`crates/ef-fat`](crates/ef-fat) | Bounded FAT12, FAT16, exFAT, and NTFS metadata parsers and extraction. |
| [`crates/ef-carve`](crates/ef-carve) | Bounded PNG, JPEG, GIF, AVI, MP4/MOV, PDF, and ZIP/Open XML structural carvers. |
| [`crates/ef-workflow`](crates/ef-workflow) | Shared scan, recovery, session, source-integrity, receipt, and export-audit workflow. |
| [`crates/ef-catalogue`](crates/ef-catalogue) | Deterministic candidate search, filtering, summaries, explanations, and bounded previews. |
| [`crates/ef-cli`](crates/ef-cli) | Command-line interface. |
| [`crates/ef-desktop`](crates/ef-desktop) | Native `eframe`/`egui` desktop workspace. |
| [`fixtures/`](fixtures/) | Deterministic synthetic source images and expected artifacts. |
| [`docs/`](docs/) | Versioned recovery contracts, architecture, safety guidance, and distribution boundaries. |

## Documentation and contribution

Start with the [project status report](docs/project-status.md), [safety and evidence boundaries](docs/safety-and-evidence.md), [architecture](docs/architecture.md), [GUI workflow](docs/gui-workflow-v1.md), [source-access architecture](docs/source-access-architecture-v1.md), [GIF windowed-discovery contract](docs/gif-window-discovery-v1.md), and [synthetic performance-control corpus](docs/performance-control-corpus-v1.md). The [contribution guide](CONTRIBUTING.md), [security policy](SECURITY.md), [code of conduct](CODE_OF_CONDUCT.md), [release process](docs/release-process.md), [controlled release decision](docs/release-decision-v1.md), [dependency advisory register](docs/dependency-advisories.md), and [changelog](CHANGELOG.md) describe how the project is maintained.

## Status

DiskTrace is a **public source project and local pre-release workspace**. Its source, deterministic fixtures, documentation, and local/hosted workflows are available for inspection. It should not be described as production-ready until the remaining manual platform acceptance, package/signing/notarization, consumer-facing artifact, authorization, and release-evidence gaps are closed.

## License

DiskTrace is licensed under the [Apache License 2.0](LICENSE).

## Responsible reporting

Use the public issue tracker for reproducible bugs and feature discussions. Do not publish real disk images, private recovered material, credentials, cryptographic keys, personal data, or active exploitation details. Report potential vulnerabilities privately using the process in [SECURITY.md](SECURITY.md).
