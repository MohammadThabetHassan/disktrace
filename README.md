<p align="center">
  <img src="docs/assets/disktrace-logo.png" width="160" alt="DiskTrace disk platter and forensic trace logo">
</p>

<h1 align="center">DiskTrace</h1>

<p align="center"><strong>Read-only forensic recovery for disk images, with explicit evidence boundaries.</strong></p>

<p align="center">
  <a href="https://github.com/MohammadThabetHassan/disktrace/actions/workflows/verify.yml"><img src="https://github.com/MohammadThabetHassan/disktrace/actions/workflows/verify.yml/badge.svg?branch=main" alt="Linux verification"></a>
  <a href="https://github.com/MohammadThabetHassan/disktrace/actions/workflows/windows-release.yml"><img src="https://github.com/MohammadThabetHassan/disktrace/actions/workflows/windows-release.yml/badge.svg?branch=main" alt="Windows distribution"></a>
  <a href="https://github.com/MohammadThabetHassan/disktrace/actions/workflows/codeql.yml"><img src="https://github.com/MohammadThabetHassan/disktrace/actions/workflows/codeql.yml/badge.svg?branch=main" alt="CodeQL"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-1f6feb.svg" alt="Apache License 2.0"></a>
</p>

DiskTrace is a local-first native desktop application for reviewing and exporting **supported** deleted or lost-file candidates from disk-image files. It is designed for people who need a guided recovery workflow and for practitioners who need traceable provenance, structural validation labels, source-integrity checks, safe export behavior, and machine-readable receipts.

> DiskTrace reads the selected source image without writing to it. It does not upload source images, recovered bytes, session manifests, telemetry, or runtime recovery data. A recovery candidate is evidence to review—not a promise that an original file is complete, authentic, safe, or legally admissible.

| Recovery workflow | Evidence workflow | Privacy posture |
| --- | --- | --- |
| Choose a local image, scan in the background, filter candidates, inspect bounded previews, and export only to a separate destination. | See the method, source range, validation state, known limits, source identity, session state, and export receipt for each supported candidate. | No cloud recovery, account system, runtime AI, telemetry, or source writes. |

## Why DiskTrace

Useful recovery software should state both **what it found** and **why it is willing to show it**. DiskTrace records how a candidate was discovered, where its bytes originate, which bounded checks passed, and where the method stops. Before export, it enforces a separate destination and creates a receipt that binds source identity, candidate facts, and output hashes.

The result is intentionally conservative. DiskTrace prefers a smaller set of explainable candidates over optimistic signature matches, silent repair, or universal-recovery claims. Read the [safety and evidence contract](docs/safety-and-evidence.md) and [architecture guide](docs/architecture.md) before relying on recovered material.

## At a glance

| Capability | What DiskTrace provides | What it deliberately does not claim |
| --- | --- | --- |
| **Local desktop recovery** | A native `eframe`/`egui` workflow for local disk-image selection, scanning, review, export, and synthetic guided demonstration. | Live-device acquisition, source writes, cloud processing, or an arbitrary-device recovery guarantee. |
| **Evidence-first review** | Method labels, source offsets, structural-validation state, source identity, session status, and export audit data. | Original filenames, folder paths, ownership, authenticity, legal admissibility, or semantic completeness. |
| **Safe exports** | Separate-destination checks, source-storage and symlink refusal, receipt-backed exports, and byte-range review. | Repair, sanitisation, malware assessment, or safe-to-open assurance for recovered content. |
| **Repeatable sessions** | A versioned local JSON manifest with SHA-256 and BLAKE3 source identity, candidate facts, and export history. | Storage of recovered payload bytes or recovery from a changed source without a new scan. |

## Supported recovery methods

DiskTrace supports narrow, documented methods rather than a generic “recover everything” mode. A listed candidate means that the stated structural checks passed; it does **not** prove that every original byte, filename, folder, or filesystem state survived deletion.

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

A quick or full format can remove filesystem metadata while leaving some raw content in place. DiskTrace can search surviving metadata and supported raw structures, so it may recover some files when no later writes occurred. It cannot guarantee every original file, filename, folder, fragment, encrypted item, overwritten sector, or SSD block affected by TRIM or controller-level garbage collection. The [safety and evidence contract](docs/safety-and-evidence.md) explains this boundary in detail.

## Start safely

DiskTrace targets Rust 2021 and the verification matrix is pinned to Rust `1.97.1`. The Linux desktop smoke test uses `xvfb-run`; ordinary desktop use does not require it.

| Step | Desktop workflow | Evidence discipline |
| --- | --- | --- |
| 1. Launch | `cargo run -p ef-desktop` | Choose a local image, not a live write target. |
| 2. Review | Scan, filter, and inspect the candidate’s method, source range, and limitation text. | A bounded preview is not content execution or media rendering. |
| 3. Export | Select an existing, separate output directory. | DiskTrace refuses source-storage, nested, symlinked, and missing destinations. |

```sh
cargo run -p ef-desktop
```

The desktop application opens with a three-step guide: choose a local image, scan and review evidence details, then recover to a separate existing folder. It also includes local-session controls and a synthetic guided demo that does not require a real image.

### Optional command-line workflows

The CLI supports repeatable evidence workflows but is not required for ordinary GUI recovery.

```sh
cargo run -p ef-cli -- inspect /path/to/image.img
cargo run -p ef-cli -- scan /path/to/image.img
cargo run -p ef-cli -- catalogue /path/to/image.img --method ntfs-contiguous
# Review the method and byte range, then copy the efc1 candidate ID from scan output.
cargo run -p ef-cli -- recover /path/to/image.img efc1-<candidate-id-from-scan> /separate/output-directory
```

## Sessions, previews, and exports

A saved session stores local source paths, dual source hashes, completed candidate details, source-integrity status, and receipt-backed export history. It never stores recovered payload bytes. If a source becomes unavailable or its length, SHA-256, or BLAKE3 changes, the historical session remains readable but recovery is blocked until the changed image is scanned as a new session.

Selected previews recheck the full saved source identity on one local handle and then read only the persisted candidate’s exact byte range. PNG, JPEG, GIF, PDF, and ZIP/Open XML candidate discovery additionally use method-specific fixed source windows with mandatory legacy parity; filesystem metadata, legacy compatibility discovery, other carvers, recovery, and exports retain their full-buffer compatibility paths. Exports intentionally retain a stricter full-source re-derivation and manifest-comparison path.

```sh
cargo run -p ef-cli -- save-session /path/to/image.img /path/to/session.disktrace.json
cargo run -p ef-cli -- session-status /path/to/session.disktrace.json
cargo run -p ef-cli -- audit-session /path/to/session.disktrace.json
cargo run -p ef-cli -- case-brief /path/to/session.disktrace.json /path/to/case-brief.md
cargo run -p ef-cli -- recover-session /path/to/session.disktrace.json efc1-<candidate-id-from-session> /separate/output-directory
```

Read the [source-access architecture](docs/source-access-architecture-v1.md), [JPEG windowed-discovery contract](docs/jpeg-window-discovery-v1.md), [GIF windowed-discovery contract](docs/gif-window-discovery-v1.md), [PDF windowed-discovery contract](docs/pdf-window-discovery-v1.md), and [ZIP/Open XML windowed-discovery contract](docs/zip-window-discovery-v1.md) for the staged design and its remaining time-of-check/time-of-use boundary.

## Verification

Run the complete deterministic local verification matrix from the workspace root:

```sh
sh scripts/verify-all.sh
```

The command checks formatting, warning-free Clippy output, workspace documentation generation, locked-dependency advisory policy, unit tests, deterministic filesystem and carving fixtures, direct and saved-session recovery, receipt-backed export auditing, source-range preview contracts, bounded PNG/JPEG/GIF/PDF/ZIP/Open XML discovery parity controls, AVI/MP4 malformed-input resilience controls, synthetic sparse/signature-dense/refusal/multi-candidate scan controls, builds, and a headless native desktop smoke launch on Linux when `xvfb-run` is available.

Every fixture is synthetic and versioned with known expected bytes and source offsets; none represents a real user image. The [synthetic performance-control corpus](docs/performance-control-corpus-v1.md) is a regression aid, not a real-device benchmark. Current local evidence and intentional limits are summarized in the [public project status report](docs/project-status.md).

## Distribution status

DiskTrace is a **public source project and local pre-release workspace**. It is not a tagged production release.

A Linux x86_64 bundle and a Windows x86_64 cross-target compatibility bundle can be built locally using the scripts in this repository. The Linux bundle has local native smoke evidence. The native hosted Windows workflow additionally verifies the portable bundle, a disposable silent installer install/uninstall path, and a retained SBOM review artifact. Hosted macOS 14 ARM64 validation builds and checks an unsigned review binary.

Those results are bounded CI evidence only. They are not a macOS package, Intel-macOS evidence, signing/notarization, SmartScreen, manual accessibility acceptance, a tagged production release, or a support SLA. Consult the [Linux distribution contract](docs/linux-distribution-v1.md), [Windows distribution contract](docs/windows-distribution-v1.md), [macOS validation contract](docs/macos-validation-v1.md), [release-candidate acceptance kit](docs/release-candidate-acceptance-kit-v1.md), and [project status report](docs/project-status.md) before sharing any build.

## Project architecture

| Path | Responsibility |
| --- | --- |
| [`crates/ef-core`](crates/ef-core) | Source identity, session model, candidate types, and recovery-method vocabulary. |
| [`crates/ef-fat`](crates/ef-fat) | Bounded FAT12, FAT16, exFAT, and NTFS metadata parsers and extraction. |
| [`crates/ef-carve`](crates/ef-carve) | Bounded PNG, JPEG, GIF, AVI, MP4/MOV, PDF, and ZIP/Open XML structural carvers. |
| [`crates/ef-workflow`](crates/ef-workflow) | Shared scan, recovery, session, source-integrity, receipt, and export-audit workflow. |
| [`crates/ef-catalogue`](crates/ef-catalogue) | Deterministic candidate search, filtering, summaries, explanations, and bounded previews. |
| [`crates/ef-cli`](crates/ef-cli) | Command-line interface for inspection, scanning, sessions, auditing, and selected-candidate recovery. |
| [`crates/ef-desktop`](crates/ef-desktop) | Native desktop workspace. |
| [`fixtures/`](fixtures/) | Deterministic synthetic source images and expected artifacts. |
| [`docs/`](docs/) | Versioned contracts, architecture, safety guidance, distribution boundaries, and release evidence. |

## Documentation and project health

| Topic | Start here |
| --- | --- |
| Scope, evidence, and architecture | [Project status](docs/project-status.md) · [Safety and evidence](docs/safety-and-evidence.md) · [Architecture](docs/architecture.md) · [GUI workflow](docs/gui-workflow-v1.md) |
| Discovery and resilience contracts | [Source-access architecture](docs/source-access-architecture-v1.md) · [GIF](docs/gif-window-discovery-v1.md) · [PDF](docs/pdf-window-discovery-v1.md) · [ZIP/Open XML](docs/zip-window-discovery-v1.md) · [AVI/MP4 resilience corpus](docs/avi-mp4-resilience-corpus-v1.md) |
| Distribution and release discipline | [Release process](docs/release-process.md) · [v0.1.0 release-candidate record](docs/release-candidate-v0.1.0.md) · [Manual-acceptance kit](docs/release-candidate-acceptance-kit-v1.md) · [Controlled release decision](docs/release-decision-v1.md) |
| Maintenance and contribution | [Contribution guide](CONTRIBUTING.md) · [Security policy](SECURITY.md) · [Code of conduct](CODE_OF_CONDUCT.md) · [Dependency advisory register](docs/dependency-advisories.md) · [Changelog](CHANGELOG.md) |

## Status and responsible reporting

DiskTrace should not be described as production-ready until the remaining manual platform acceptance, package/signing/notarization, consumer-facing artifact, authorization, and release-evidence gaps are closed. The current [controlled release decision](docs/release-decision-v1.md) is intentionally a no-go record, not publication authorization.

Use the public issue tracker for reproducible bugs and feature discussions. Do not publish real disk images, private recovered material, credentials, cryptographic keys, personal data, or active exploitation details. Report potential vulnerabilities privately through the process in [SECURITY.md](SECURITY.md).

## License

DiskTrace is licensed under the [Apache License 2.0](LICENSE).
