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
| PNG, JPEG, and GIF discovery | PNG, JPEG, and GIF structural discovery use fixed 1 MiB primary source windows, method-specific signature overlap and ownership, bounded file-backed validation, and legacy-candidate parity enforcement. JPEG retains the existing 128 MiB structural cap; GIF retains the existing 64 MiB structural cap. | The full source is still buffered for filesystem metadata, legacy compatibility discovery, AVI/MP4/PDF/ZIP carving, recovery, and exports; this is not a complete streaming-scan or parser-level cancellation claim. |
| Exports and audit | Exports require an approved separate destination and produce receipt-backed records; saved output can be checked later against recorded hashes. | A receipt records a local integrity observation, not original-file authenticity or legal admissibility. |
| Session schema | Session manifests reject unsupported schema versions and unrecognized top-level fields. | Nested future-format changes still require explicit compatibility design and versioned tests. |
| Privacy | No runtime cloud recovery, telemetry, account system, or source write behavior is part of the application workflow. | The operator remains responsible for handling sensitive images and recovered content safely. |

## Verification and governance evidence

The repository exposes a deterministic local matrix through `sh scripts/verify-all.sh`. The matrix includes formatting, strict Clippy, rustdoc warnings-as-errors, dependency advisory policy, workspace tests, desktop UI contracts, filesystem and carving fixtures, saved-session and receipt-audit scenarios, source-range preview regressions, synthetic sparse/signature-dense/refusal/multi-candidate scan controls, a build, and a Linux native desktop smoke launch when the local environment supports `xvfb-run`.[2]

The selected-preview source-access increment is covered by exact-byte, zero-length EOF, changed-source, same-length substituted-source, overflow/out-of-bounds, cancellation, workflow-membership, cross-fixture parity, and desktop-worker regressions. Exports intentionally retain a stricter full-source re-derivation and manifest-comparison workflow.[4]

A scan now checks cancellation before source work, between 1 MiB full-buffer reads, after each completed legacy discovery method stage, and throughout bounded PNG, JPEG, and GIF window reads. A cancellation preserves an existing completed catalogue and applies no new result. These are cooperative stage checkpoints, not parser-loop cancellation, a byte-accurate progress indicator, a bounded in-stage latency promise, whole-scan streaming, or a complete time-of-check/time-of-use solution.[16]

A deterministic synthetic scan-control corpus now defines sparse, signature-dense acceptance, signature-dense refusal, and multi-candidate PNG scenarios. The ordinary matrix generates every scenario and asserts its declared candidate count and exact PNG offset geometry; the separate local measurement harness records timings only after the same assertions pass. This is a compatibility and regression control for the existing full-buffer path only, not a benchmark for real storage devices, production throughput, fragmented filesystems, cache state, hostile-input resilience, or a streaming implementation.[15]

| Validation area | Current result | What the result does not mean |
| --- | --- | --- |
| Linux desktop | Local bundle and native desktop smoke passed; the exact hosted Ubuntu 24.04 `Verify` workflow passed the bounded PNG/JPEG discovery, legacy-stage cancellation, and recovery matrix.[9] | It is not a signed production installer or a general Linux distribution claim. |
| Windows path | The exact hosted native Windows distribution workflow passed formatting, linting, tests, the synthetic scan-control corpus, portable bundle verification, installer creation/checksums, a disposable silent installer install/uninstall acceptance gate, pinned CycloneDX SBOM generation, and review-artifact upload.[12] | It is not SmartScreen, code-signing, independent-user usability, GUI/accessibility acceptance, upgrade coverage, or broad hardware compatibility evidence. |
| macOS | The exact hosted macOS 14 ARM64 workflow passed formatting, strict linting, workspace tests, release desktop-binary build, ARM64 identity check, checksum creation, and unsigned review-artifact upload.[11] | It is not Intel-macOS evidence, a `.app` bundle, installer, signing, notarization, Gatekeeper acceptance, manual usability evidence, or a general macOS distribution claim. |
| Release governance | `main` requires the exact `Rust workspace and recovery fixtures`, `Windows x86_64 bundle and installer`, `CodeQL Rust analysis`, and `macOS 14 ARM64 workspace validation` contexts; it also requires an up-to-date branch, one current CODEOWNERS review for pull requests, linear history, and resolved conversations, and blocks force pushes and deletion. Exact hosted Linux, Windows, macOS 14 ARM64, and CodeQL workflows passed for `127cbadbe36a2e91644165ba94168cbdc9e60a32`.[9] [10] [11] [12] | No semantic tag, signed release artifact, GitHub Release, macOS package/manual-acceptance evidence, or release provenance exists. |
| Dependency review | Dependabot security updates are enabled. Patch/minor updates are bounded and grouped; major updates are separate review decisions. | A passing dependency PR is not automatically merged or treated as a release decision. |
| Code scanning | The least-privilege Rust CodeQL workflow completed successfully for exact revision `127cbadbe36a2e91644165ba94168cbdc9e60a32`.[10] | A successful scan is not a security certification, a proof that all vulnerabilities are absent, or a production-release claim. |
| SBOM transparency | The exact hosted Windows workflow generated nine CycloneDX 1.5 dependency documents for all Cargo targets, SHA-256 checksums, and commit/generator metadata using pinned `cargo-cyclonedx 0.5.9`.[12] [13] | The retained CI review artifact is not an attestation, signed provenance statement, vulnerability-free claim, GitHub Release asset, or supported download. |

## Current readiness assessment

The current strict interim assessment is **85/100 for local product and portfolio quality**, **90/100 for public repository quality**, and **64/100 for public-release readiness**. The public-repository score recognizes bounded PNG and JPEG discovery, forensic safety controls, deterministic verification depth, a synthetic acceptance/refusal corpus, method-stage cancellation parity controls, protected four-context branch governance, hosted Linux/Windows/macOS ARM64/CodeQL evidence, native Windows installer install/uninstall acceptance, and generated SBOM review evidence. Release readiness remains deliberately lower because only two discovery methods are windowed while most discovery, recovery, and export paths remain full-buffer compatibility paths; parser loops still have no interruption/progress controls; macOS packaging/manual-acceptance and signing evidence are absent; the SBOM is not an attestation; and no semantic version or release provenance exists.[6] [13]

| Assessment | Meaning |
| --- | --- |
| **Strong public pre-release evidence** | The documented local workflows, protected-branch controls, exact hosted Linux/Windows/macOS 14 ARM64/CodeQL checks, native Windows installer mechanics, and SBOM review artifact can be inspected within their declared scope. |
| **Not production-ready** | macOS package/manual-acceptance evidence, cross-platform signing/notarization, manual installer/accessibility acceptance, a semantic version tag, release assets, immutable consumer-facing provenance/attestation, and release provenance remain outstanding. |
| **Not universal recovery** | The project intentionally refuses broad classes of unsupported or ambiguous recovery situations instead of claiming that every deleted or formatted file can be restored. |

## Highest-value next work

The completed PNG, JPEG, and GIF increments now have method-specific fixed-window discovery contracts, primary-window ownership, boundary/refusal controls, completed-window cancellation, and legacy candidate parity. JPEG retains the legacy absolute 128 MiB structural cap while scanning entropy data through bounded sequential file reads; GIF retains the legacy absolute 64 MiB cap while scanning its supported block stream through bounded sequential file reads. Legacy discovery also observes cooperative cancellation between completed method stages, while retaining an explicit non-claim for parser-loop interruption and progress reporting. A four-scenario synthetic performance corpus makes sparse, signature-dense, refusal-heavy, and multi-candidate regression behavior visible. The next source-access work is a separately designed migration for another method; no whole-scan streaming claim is justified yet.[15] [16] [17] [18]

The FAT32 feasibility assessment now explicitly defers a deleted-entry recovery claim. Any future FAT32 increment must first establish conservative geometry parsing, root-directory chain handling, 28-bit FAT allocation rules, contiguous-range constraints, refusal cases, and deterministic positive/refusal fixtures.[7] The completed release-engineering increments now include macOS 14 ARM64 hosted validation, Windows installer mechanics, CodeQL, SBOM review artifacts, and a cross-platform synthetic scan-control corpus; the next work is additional bounded source access, support-scoped manual acceptance, macOS packaging, and an explicit versioned release decision only after separately authorized target checks are green. Until those items are complete, source publication should be understood as an invitation to inspect and contribute—not as a production-release guarantee.

Maintainer governance is now documented in a versioned runbook, structured public issue forms reject sensitive case material and require evidence-led reporting, and a manual desktop acceptance checklist defines the observations needed before a platform-specific acceptance record can be made. These controls improve reviewability; they do **not** constitute a completed manual acceptance, accessibility certification, signed release, or broader platform claim.[8] The current controlled decision package records the verified pre-release target and explicitly blocks tag, release, asset, signing, notarization, and support-level actions pending separate owner authorization.[14]

## How to help responsibly

Contributors can begin with the [contribution guide](../CONTRIBUTING.md) and should read the [security policy](../SECURITY.md) before reporting vulnerabilities. Do not attach real disk images, recovered files, credentials, cryptographic keys, or personal data to public issues. Use synthetic, minimized reproductions whenever possible. Recovery-method expansion should add a bounded contract, positive and refusal fixtures, documentation, and deterministic verification before making user-facing claims.

## References

[1]: ../README.md "DiskTrace README"
[2]: ../scripts/verify-all.sh "Complete local verification matrix"
[3]: safety-and-evidence.md "Safety and evidence boundary"
[4]: source-access-architecture-v1.md "Source-access architecture v1"
[5]: ../scripts/measure-large-sparse-scan.sh "Deterministic sparse-control measurement harness"
[6]: release-process.md "Release process and current evidence boundary"
[7]: fat32-feasibility-v1.md "FAT32 recovery feasibility boundary"
[8]: maintainer-runbook-v1.md "Maintainer operating contract"
[9]: https://github.com/MohammadThabetHassan/disktrace/actions/runs/32834283789 "Hosted Linux verification for 127cbad"
[10]: https://github.com/MohammadThabetHassan/disktrace/actions/runs/32834283813 "Hosted CodeQL analysis for 127cbad"
[11]: https://github.com/MohammadThabetHassan/disktrace/actions/runs/32834283785 "Hosted macOS 14 ARM64 validation for 127cbad"
[12]: https://github.com/MohammadThabetHassan/disktrace/actions/runs/32834283829 "Hosted Windows distribution, installer acceptance, synthetic corpus, and SBOM review artifact for 127cbad"
[13]: sbom-provenance-v1.md "SBOM and provenance contract v1"
[14]: release-decision-v1.md "Controlled release decision package v1"
[15]: performance-control-corpus-v1.md "Synthetic scan-control corpus v1"
[16]: legacy-discovery-cancellation-v1.md "Legacy discovery cancellation v1"
[17]: jpeg-window-discovery-v1.md "JPEG windowed discovery v1"
[18]: gif-window-discovery-v1.md "GIF windowed discovery v1"
