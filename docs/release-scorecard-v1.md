# DiskTrace Release Scorecard v1

## Purpose

This scorecard prevents DiskTrace from being described as release-grade merely because source is public or a local build succeeds. A public release requires all applicable evidence in this document to be complete on one exact target commit.

> A public release is an explicit, versioned decision. It is not a CI artifact, a local archive, a draft note, or a pushed source commit.

## Current baseline

| Dimension | Current posture | Release-grade target |
|---|---|---|
| Product and forensic scope | Bounded recovery methods, source identity, session, receipt, separate-destination, and refusal contracts are implemented; AVI/MP4 resilience controls cover minimized malformed declarations, non-suppression, and ordering. | New methods or performance claims are added only with method contracts, deterministic positive/refusal controls, and public scope wording. |
| Local verification | The full local matrix is deterministic and includes quality, fixture, audit, UI, sparse-control, build, and desktop-smoke checks. | The matrix passes on the exact target commit and any new capability adds enforceable parity and boundary tests. |
| Hosted verification | Exact `f70f2f517a56ffe8ed4fadc654e47eb9a421b3cb` Linux, native Windows installer/SBOM, macOS 14 ARM64, and Rust CodeQL workflows passed.[1] [2] [3] [4] | The exact tagged target must repeat every required context; a previous green SHA is not tag-target evidence. |
| Governance | `main` requires the four exact Linux, Windows, macOS ARM64, and CodeQL contexts; current pull requests require CODEOWNERS review; linear history, resolved conversations, and force-push/deletion blocks are active. | Workflow permissions, security/update policy, release ownership, and failure-triage procedure remain documented and current. |
| Distribution | Linux and Windows package contracts exist; Windows passed disposable installer mechanics and generated a retained SBOM review artifact; macOS 14 ARM64 produces an unsigned review binary. | Versioned consumer artifacts, support-scoped manual acceptance records, public release assets, provenance where authorized, and signing status are recorded. |
| Public release | No semantic tag or GitHub Release exists. | An explicitly authorized annotated tag and public release resolve to the verified release target and approved notes. |

## Release target record

Before requesting a release decision, create a release record with the following fields.

| Field | Required value |
|---|---|
| Intended version | Semantic version selected for the actual material change set. |
| Target commit | Full SHA on protected `main`; no uncommitted workspace changes. |
| Authorized identity | Git author and committer identity explicitly authorized by the owner. |
| Local evidence | Executed commands, timestamps, and concise results for the full matrix and artifact verifiers. |
| Hosted evidence | URLs and successful conclusions for each exact-target required workflow. |
| Artifact record | Platform, architecture, filename, byte size, SHA-256, package verifier result, and support status. |
| Advisory disposition | Current `cargo audit` result and reviewed maintenance/advisory entries. |
| Manual acceptance | Exact platform, OS/runner, workflow path, installer/accessibility checks, and any not-performed items. |
| Release notes | Supported recovery methods, material changes, known limits, migration impact, and security/signing status. |

## Required release gates

| Gate | Required evidence | Current status before first public release |
|---|---|---|
| Recovery safety | Method contracts, refusal conditions, source read-only behavior, destination policy, export receipts, and scope text match code. | Bounded public source evidence exists; required for every release. |
| Code quality | Formatter, strict Clippy, rustdoc warnings-as-errors, tests, build, and `sh scripts/verify-all.sh` pass. | Passed on the source tree that became `f70f2f5`; must rerun on a release target. |
| Dependency and code security | `cargo audit`, reviewed advisory register, configured code scanning, and no committed credentials or sensitive artifacts. | `cargo audit`/CodeQL are part of evidence; a release still requires target-specific advisory review. |
| Hosted Linux | Exact-target Linux Verify, including the native smoke gate, passes. | Passed for `f70f2f5`; no tag-target evidence exists. |
| Hosted Windows | Exact-target Windows distribution, package verification, installer creation/checksums, bounded installer mechanics, SBOM review artifact, and upload pass. | Passed for `f70f2f5`; no public versioned artifact exists. |
| macOS | Native build/package/acceptance evidence exists for every advertised macOS target. | macOS 14 ARM64 build/test/review-binary evidence exists; package, Intel scope, signing/notarization, and manual acceptance do not. |
| Artifact acceptance | Platform-specific bundle and installer checks plus documented manual acceptance where automation is unavailable. | Windows automated installer mechanics exist; the preparation-only [manual-acceptance kit](release-candidate-acceptance-kit-v1.md) exists, but support-scoped manual acceptance remains required. |
| Supply chain | Checksums, SBOM/provenance where enabled, least-privilege workflows, and signing status are recorded. | SBOM review evidence/checksums exist; consumer-facing attestation, versioned assets, and signing are absent. |
| Governance | Protected branch, real required contexts, security route, dependency policy, and maintainer release process are active. | Active; the direct-main owner model remains documented. |
| Publication | Owner explicitly authorizes annotated tag, release publication, assets, and signing use. | **Not authorized.** |

## Scoring rule

A score above **90/100** requires the product, evidence, documentation, public presentation, governance, and release record to reinforce each other. DiskTrace must not claim a strict 90+ public-release readiness score if a required platform is advertised without evidence, if the release target lacks green exact-SHA workflows, if release assets cannot be traced to the source revision, or if the recovery boundary is overstated.

A missing credential for signing/notarization does not justify a simulated claim. In that case DiskTrace may improve product and repository quality, but its public-release-readiness score remains below the threshold and the supported-platform list must be narrowed accordingly.

## Authorization boundary

The owner must explicitly authorize the following actions after this scorecard is complete: semantic tag creation, GitHub Release publication, release asset upload, code-signing certificate or notarization credential use, and any public support-level change. No automated workflow may make those decisions on the owner’s behalf.

## References

[1]: https://github.com/MohammadThabetHassan/disktrace/actions/runs/33007849882 "Hosted Linux verification for f70f2f5"
[2]: https://github.com/MohammadThabetHassan/disktrace/actions/runs/33007849876 "Hosted Windows distribution, installer mechanics, and SBOM review artifact for f70f2f5"
[3]: https://github.com/MohammadThabetHassan/disktrace/actions/runs/33007849902 "Hosted macOS 14 ARM64 validation for f70f2f5"
[4]: https://github.com/MohammadThabetHassan/disktrace/actions/runs/33007849872 "Hosted CodeQL analysis for f70f2f5"
