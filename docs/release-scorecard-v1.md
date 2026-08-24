# DiskTrace Release Scorecard v1

## Purpose

This scorecard prevents DiskTrace from being described as release-grade merely because source is public or a local build succeeds. A public release requires all applicable evidence in this document to be complete on one exact target commit.

> A public release is an explicit, versioned decision. It is not a CI artifact, a local archive, a draft note, or a pushed source commit.

## Current baseline

| Dimension | Current posture | Release-grade target |
| --- | --- | --- |
| Product and forensic scope | Bounded recovery methods, source identity, session, receipt, and refusal contracts are implemented. | New methods or performance claims are added only with method contracts, deterministic positive/refusal controls, and public scope wording. |
| Local verification | The full local matrix is deterministic and includes quality, fixture, audit, UI, sparse-control, build, and desktop-smoke checks. | The matrix passes on the exact target commit and any new capability adds its own enforceable parity and boundary tests. |
| Hosted verification | Protected `main` has green Linux Verify and Windows distribution evidence. | The exact tagged target has green Linux, Windows, and any advertised macOS checks. |
| Governance | Required current-branch Linux/Windows checks, linear history, conversation resolution, and force-push/deletion blocks are configured. | Workflow permissions, security/update policy, release ownership, and failure-triage procedure remain documented and current. |
| Distribution | Linux and Windows package contracts exist; Windows produces reviewed CI artifacts. | Versioned artifacts, checksums, platform acceptance records, release assets, provenance where enabled, and signing status are recorded. |
| Public release | No semantic tag or GitHub Release exists. | An authorized annotated tag and public release resolve to the verified release target and approved notes. |

## Release target record

Before requesting a release decision, create a release record with the following fields.

| Field | Required value |
| --- | --- |
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

| Gate | Required evidence | Status before first public release |
| --- | --- | --- |
| Recovery safety | Method contracts, refusal conditions, source read-only behavior, destination policy, export receipts, and scope text match code. | Required each release. |
| Code quality | Formatter, strict Clippy, rustdoc warnings-as-errors, tests, build, and `sh scripts/verify-all.sh` pass. | Required each release. |
| Dependency and code security | `cargo audit` plus reviewed advisory register; configured code scanning once introduced; no committed credentials or sensitive artifacts. | Advisory review exists; scanning expansion remains planned. |
| Hosted Linux | Exact-target Linux Verify, including the native smoke gate, passes. | Existing evidence is not yet tag-target evidence. |
| Hosted Windows | Exact-target Windows distribution, package verification, installer creation, checksums, and review artifact upload pass. | Existing evidence is not yet tag-target evidence. |
| macOS | Native build/package/acceptance evidence exists for every advertised macOS target. | Not implemented; blocks a macOS support claim. |
| Artifact acceptance | Platform-specific bundle and installer checks plus documented manual acceptance where automation is unavailable. | Windows installer acceptance and macOS evidence remain planned. |
| Supply chain | Checksums, SBOM/provenance where enabled, least-privilege workflows, and signing status are recorded. | Release provenance and signing plan remain planned. |
| Governance | Protected branch, real required contexts, security route, dependency policy, and maintainer release process are active. | Partially complete; maintainer runbook remains planned. |
| Publication | Owner explicitly authorizes annotated tag, release publication, assets, and signing use. | Not authorized. |

## Scoring rule

A score above **90/100** requires the product, evidence, documentation, public presentation, governance, and release record to reinforce each other. DiskTrace must not claim a strict 90+ public-release readiness score if a required platform is advertised without evidence, if the release target lacks green exact-SHA workflows, if release assets cannot be traced to the source revision, or if the recovery boundary is overstated.

A missing credential for signing/notarization does not justify a simulated claim. In that case DiskTrace may improve product and repository quality, but its public-release-readiness score remains below the threshold and the supported-platform list must be narrowed accordingly.

## Authorization boundary

The owner must explicitly authorize the following actions after this scorecard is complete: semantic tag creation, GitHub Release publication, release asset upload, code-signing certificate or notarization credential use, and any public support-level change. No automated workflow may make those decisions on the owner’s behalf.
