# DiskTrace Project Status

**Current designation:** public source project; protected, verified pre-release workspace.

DiskTrace is a local-first forensic recovery application for examining disk-image files, reviewing supported deleted-file candidates, and exporting selected candidates to a separate destination. Its design favors explicit evidence and refusal conditions over broad recovery promises. The product’s current boundaries, method contracts, and verification entry point are maintained in the repository rather than inferred from marketing language.[1] [2]

> A successful candidate means that the listed method-specific checks accepted a bounded byte range. It does not establish original filename, path, completeness, authenticity, malware safety, evidentiary admissibility, or recovery of every deleted file.

## What is available today

The native desktop workflow provides a guided path to choose a local image, scan in the background, examine a candidate catalogue, review bounded preview information, and export a selected candidate only after a separate-destination check. The optional CLI supports repeatable inspection, scanning, filtering, session handling, export auditing, and case briefs. Both workflows retain local-only handling: no source image or recovered payload is uploaded by the application.[1] [3]

| Capability | Current evidence | Intentional boundary |
| --- | --- | --- |
| Source identity | Source length, SHA-256, and BLAKE3 are recorded for scans and rechecked for saved-session recovery. | Identity checks identify source changes; they do not prove a source is an original forensic acquisition. |
| Supported recovery | FAT12, FAT16, exFAT, NTFS resident, narrow NTFS contiguous, and structural carving for PNG, JPEG, GIF, AVI, self-contained MP4/MOV, PDF, and ZIP/Open XML have deterministic fixture coverage. | Unsupported, fragmented, overwritten, encrypted, TRIM-affected, controller-discarded, and ambiguous cases are refused or remain out of scope. |
| Selected previews | The selected candidate is rechecked against the saved source identity and read by exact byte range instead of rebuffering the entire image for that preview. | Discovery and exports retain compatibility paths where they have not received a separate source-access migration; the preview design does not claim a complete TOCTOU solution. |
| PNG discovery | PNG structural discovery uses fixed primary source windows with signature overlap, primary-window ownership, bounded header reads, and legacy-candidate parity enforcement. | The full source is still buffered for filesystem metadata and all non-PNG carving; this is not a complete streaming-scan or parser-level cancellation claim. |
| Exports and audit | Exports require an approved separate destination and produce receipt-backed records; saved output can be checked later against recorded hashes. | A receipt records a local integrity observation, not original-file authenticity or legal admissibility. |
| Session schema | Session manifests reject unsupported schema versions and unrecognized top-level fields. | Nested future-format changes still require explicit compatibility design and versioned tests. |
| Privacy | No runtime cloud recovery, telemetry, account system, or source write behavior is part of the application workflow. | The operator remains responsible for handling sensitive images and recovered content safely. |

## Verification and governance evidence

The repository exposes a deterministic local matrix through `sh scripts/verify-all.sh`. The matrix includes formatting, strict Clippy, rustdoc warnings-as-errors, dependency advisory policy, workspace tests, desktop UI contracts, filesystem and carving fixtures, saved-session and receipt-audit scenarios, source-range preview regressions, a controlled sparse-image control, a build, and a Linux native desktop smoke launch when the local environment supports `xvfb-run`.[2]

The selected-preview source-access increment is covered by exact-byte, zero-length EOF, changed-source, same-length substituted-source, overflow/out-of-bounds, cancellation, workflow-membership, cross-fixture parity, and desktop-worker regressions. Exports intentionally retain a stricter full-source re-derivation and manifest-comparison workflow.[4]

A deterministic 64 MiB sparse control records one expected PNG candidate at a fixed offset across three runs. Its observed average duration was 7,537.817 ms, or 8.49 MiB/s in that synthetic setup. This is a regression and compatibility observation only; it is not a benchmark for real storage devices, production throughput, signature-dense images, fragmented files, or a streaming implementation.[5]

| Validation area | Current result | What the result does not mean |
| --- | --- | --- |
| Linux desktop | Local bundle and native desktop smoke passed; the exact hosted Ubuntu 24.04 `Verify` workflow passed after installing the required XKB runtime dependency. | It is not a signed production installer or a general Linux distribution claim. |
| Windows path | The exact hosted native Windows distribution workflow passed formatting, linting, tests, portable bundle verification, installer creation, checksums, and review-artifact upload. | It is not SmartScreen, code-signing, independent-user usability, or broad hardware compatibility evidence. |
| macOS | No validation has been performed. | No macOS compatibility or distribution claim is made. |
| Release governance | `main` is protected by the exact Linux and Windows quality contexts, requires an up-to-date branch, linear history, and resolved conversations, and blocks force pushes and deletion. Hosted Linux and Windows workflows passed on revision `dc48828`. | No semantic tag, signed release artifact, GitHub Release, macOS evidence, or release provenance exists. |
| Dependency review | Dependabot security updates are enabled. Patch/minor updates are bounded and grouped; major updates are separate review decisions. | A passing dependency PR is not automatically merged or treated as a release decision. |

## Current readiness assessment

The current strict assessment is **83/100 for local product and portfolio quality**, **84/100 for public repository quality**, and **56/100 for public-release readiness**. The public-repository score recognizes the bounded recovery methods, forensic safety controls, native workflow, deterministic verification depth, protected branch governance, and exact hosted Linux and Windows evidence. Release readiness remains deliberately lower because discovery is not windowed, parser loops do not yet have cancellation/progress controls, macOS validation and signing are absent, and no versioned release provenance exists.[6]

| Assessment | Meaning |
| --- | --- |
| **Strong public pre-release evidence** | The documented local workflows, protected-branch controls, and exact hosted Linux/Windows checks can be inspected and reproduced within their declared scope. |
| **Not production-ready** | macOS evidence, cross-platform signing/notarization, manual installer/accessibility acceptance, a semantic version tag, release assets, and release provenance remain outstanding. |
| **Not universal recovery** | The project intentionally refuses broad classes of unsupported or ambiguous recovery situations instead of claiming that every deleted or formatted file can be restored. |

## Highest-value next work

The completed PNG increment now has a fixed-window discovery contract, primary-window ownership, boundary/refusal controls, cancellation after a completed window, and legacy candidate parity. The next source-access work is a larger controlled corpus and a separately designed migration for another method; no whole-scan streaming claim is justified yet.

The next recovery-method decision is a feasibility-gated FAT32 deleted-entry increment with conservative contiguous/allocation requirements and deterministic positive/refusal fixtures. The next release-engineering increment is macOS build/package validation, Windows installer acceptance, supply-chain provenance, and an explicit versioned release decision after exact target checks are green. Until those items are complete, source publication should be understood as an invitation to inspect and contribute—not as a production-release guarantee.

## How to help responsibly

Contributors can begin with the [contribution guide](../CONTRIBUTING.md) and should read the [security policy](../SECURITY.md) before reporting vulnerabilities. Do not attach real disk images, recovered files, credentials, cryptographic keys, or personal data to public issues. Use synthetic, minimized reproductions whenever possible. Recovery-method expansion should add a bounded contract, positive and refusal fixtures, documentation, and deterministic verification before making user-facing claims.

## References

[1]: ../README.md "DiskTrace README"
[2]: ../scripts/verify-all.sh "Complete local verification matrix"
[3]: safety-and-evidence.md "Safety and evidence boundary"
[4]: source-access-architecture-v1.md "Source-access architecture v1"
[5]: ../scripts/measure-large-sparse-scan.sh "Deterministic sparse-control measurement harness"
[6]: release-process.md "Release process and current evidence boundary"
