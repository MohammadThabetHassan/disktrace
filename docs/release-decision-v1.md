# DiskTrace controlled release decision package v1

## Decision summary

**Decision: no public-release action.** This record identifies `afc5bb20148dd1f8845312d28f1f86e2d43204a6` as a verified pre-release source target for maintainer review. It does not authorize a semantic version, annotated tag, GitHub Release, release asset upload, signing, notarization, support-level change, or production-release statement.

> A green CI source revision and a review artifact are evidence for a bounded engineering decision. They are not a public release, a signed distribution, or proof of universal recovery capability.

| Field | Recorded value |
|---|---|
| Repository | `MohammadThabetHassan/disktrace` (public) |
| Source target | `afc5bb20148dd1f8845312d28f1f86e2d43204a6` on protected `main` |
| Commit identity | Author and committer: `MohammadThabetHassan <20220002188@students.cud.ac.ae>` |
| Local gate | `sh scripts/verify-all.sh` passed on the source tree that became this target. |
| Hosted required contexts | Linux verification, Windows distribution, macOS 14 ARM64 validation, and Rust CodeQL all passed on this exact SHA.[1] [2] [3] [4] |
| Current disposition | **Pre-release evidence complete for declared scope; publication remains blocked.** |
| Owner authorization | Not granted for tag creation, release publication, assets, signing, notarization, or public support claim. |

## Exact evidence record

| Evidence area | Result | Scope and limit |
|---|---|---|
| Local quality matrix | Passed. The matrix covers formatting, strict linting, rustdoc, dependency audit, workspace tests, deterministic recovery/refusal fixtures, static documentation contracts, build, and the bounded Linux desktop smoke when supported. | It is local validation, not a substitute for user acceptance or a signed release. |
| Hosted Linux | `Rust workspace and recovery fixtures` passed on the exact target.[1] | It is not a signed Linux installer or broad distribution evidence. |
| Hosted Windows | `Windows x86_64 bundle and installer` passed, including ZIP verification, Inno Setup build/checksums, disposable silent installer install/uninstall mechanics, and pinned CycloneDX SBOM review-artifact generation.[2] | It does not validate SmartScreen, code signing, upgrades, real-user GUI behavior, accessibility, or all Windows hardware. |
| Hosted macOS | `macOS 14 ARM64 workspace validation` passed, including build, tests, ARM64 check, checksum, and unsigned review-artifact upload.[3] | It is not Intel macOS, a `.app` package, a signed/notarized application, Gatekeeper evidence, or manual acceptance. |
| Code scanning | `CodeQL Rust analysis` passed with its configured Rust build-free extraction mode.[4] | It is not a security certification or an assertion that all defects or vulnerabilities are absent. |
| SBOM review artifact | The Windows workflow generated nine CycloneDX 1.5 JSON documents for all Cargo targets, document checksums, and source/generator metadata with `cargo-cyclonedx 0.5.9`.[2] [5] | The CI review artifact is not an attestation, signing record, public release asset, or supported download. |
| Governance | Protected `main` requires the exact Linux, Windows, macOS ARM64, and CodeQL contexts; pull requests require one current CODEOWNERS review, while the authorized owner direct-main path remains available. | Direct-main bypass notices are an intentional contribution-model limitation, not evidence of independent review. |

## Scope integrity

The source target remains a local-first forensic recovery workspace. Selected sources are read only; source identity is checked; exports require a separate destination and generate receipts; no runtime cloud recovery, telemetry, account system, or runtime AI path is introduced. Supported methods and refusal conditions remain bounded by the public contracts. In particular, the project does not claim universal recovery, fragmented-file recovery, overwritten/TRIM-affected recovery, encrypted-data recovery, or original-file authenticity.

The current source-access evidence is also limited: PNG, JPEG, GIF, PDF, and ZIP/Open XML discovery use bounded primary windows with legacy-parity enforcement, while filesystem metadata, AVI/MP4/MOV discovery, recovery, export, and audit rederivation retain compatibility paths that buffer the full source. This package does not turn the bounded discovery increments into a whole-scan streaming or complete TOCTOU claim.

## Publication blockers and owner decisions

| Gate | Current state | Required action before publication |
|---|---|---|
| Explicit publication authorization | **Blocked.** The owner has not authorized tag, release page, release assets, signing, notarization, or a public support-level change. | Obtain a separate explicit instruction that identifies the intended semantic version, target SHA, release visibility, asset policy, and permitted signing/notarization actions. |
| Version and immutable release record | **Not started.** No semantic tag or GitHub Release exists. | Select an intended version, prepare final notes, create an annotated tag only after owner authorization, and bind every release link/checksum to that exact target. |
| Consumer-facing artifacts | **Not ready.** Current platform files are short-retention CI review artifacts, not public release assets. | Build and record versioned Linux x86_64 and Windows x86_64 artifacts, byte sizes, checksums, and support status for the authorized release target; macOS remains excluded unless its separate evidence gates are completed. |
| Windows acceptance and signing | **Partially evidenced.** One clean hosted runner passed bounded installer mechanics; signing is absent. | Use the preparation-only [manual-acceptance kit](release-candidate-acceptance-kit-v1.md) to complete the required native Windows manual record for the intended release scope, then use Authenticode only if the owner separately authorizes a credential and signing process. |
| macOS packaging and acceptance | **Blocked for macOS distribution.** ARM64 hosted validation exists, but no `.app`/installer, manual acceptance, signing, notarization, or Intel evidence exists. | Narrow the advertised platform list or complete package, manual, signing, and notarization evidence under separately authorized Apple credentials. |
| Supply-chain provenance | **Partially evidenced.** Review SBOM and checksums exist; no consumer-facing immutable provenance or attestation exists. | Decide whether a release artifact needs attestation, define verification and lifecycle policy, and authorize the additional least-privilege permissions only for that release workflow. |
| Manual accessibility acceptance | **Not completed.** A checklist and preparation-only kit exist, not an observed certification or platform record. | Perform and record the declared keyboard, failure, preview, export, audit, and platform observations for the exact release candidate on every advertised platform. |
| Dependency changes | **Deferred.** Dependabot PRs #10 (patch/minor group) and #11 (`thiserror` major) are not merged by this decision package. | Reinspect against current `main`; only apply an explicitly approved, compatibility-validated update under the authorized commit identity. |
| Recovery scope | **Bounded.** FAT32 deleted-file recovery remains deferred; no unsupported recovery method is represented as available. | Add methods only with parser contracts, positive/refusal fixtures, deterministic verification, and scope documentation. |

## Release procedure if separately authorized

If the owner later grants explicit publication authorization, run the release process from a clean, current branch and replace this no-go record with a target-specific decision. The procedure must rerun `sh scripts/verify-all.sh`, generate final intended artifacts and checksums, run every required hosted context on the exact target, verify manual acceptance claims, record dependency/advisory disposition, and review the final release notes. Only then may the explicitly authorized annotated tag, release page, assets, or signing/notarization action be performed.[6] [7]

The owner’s authorization must be new and specific. It must not be inferred from source-commit permission, from a green workflow, or from this document.

## References

[1]: https://github.com/MohammadThabetHassan/disktrace/actions/runs/32937721920 "Hosted Linux verification for afc5bb2"
[2]: https://github.com/MohammadThabetHassan/disktrace/actions/runs/32937721931 "Hosted Windows distribution, installer mechanics, and SBOM review artifact for afc5bb2"
[3]: https://github.com/MohammadThabetHassan/disktrace/actions/runs/32937722003 "Hosted macOS 14 ARM64 validation for afc5bb2"
[4]: https://github.com/MohammadThabetHassan/disktrace/actions/runs/32937722018 "Hosted CodeQL analysis for afc5bb2"
[5]: sbom-provenance-v1.md "DiskTrace SBOM and provenance contract v1"
[6]: release-process.md "DiskTrace release process"
[7]: release-scorecard-v1.md "DiskTrace Release Scorecard v1"
