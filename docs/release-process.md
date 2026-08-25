# DiskTrace Release Process

This document defines DiskTrace’s release gate. DiskTrace source is public, `main` is protected, and the repository has exact-SHA hosted Linux, Windows, macOS 14 ARM64, and CodeQL verification evidence. Those facts do **not** make the project a tagged, signed, or production release. A local archive, CI review artifact, or pushed commit is not a public release.

## Current release posture

| Evidence area | Current state | Remaining boundary |
| --- | --- | --- |
| Source and governance | Public repository, authorized commit identity, protected `main`, required current-branch checks, linear history, and force-push/deletion blocks. | The authorized owner can use the documented direct-main path; external pull-request merges must satisfy the required checks. |
| Hosted verification | Exact hosted Ubuntu Linux Verify, native Windows distribution, macOS 14 ARM64 validation, and Rust CodeQL workflows passed on the protected pre-release revision. | A release must repeat every required context on the exact tagged target; passing a previous revision is not release evidence. |
| Local quality | `sh scripts/verify-all.sh` executes format, strict linting, rustdoc, `cargo audit`, workspace tests, deterministic recovery fixtures, build, and Linux desktop smoke where supported. | Local evidence does not substitute for release-target hosted evidence or native platform acceptance. |
| Distribution | Linux and Windows package contracts exist; Windows has hosted installer-mechanics and SBOM review-artifact evidence; macOS 14 ARM64 produces an unsigned review binary. | No public release assets, consumer-facing immutable provenance/attestation, production code signing, macOS package, or notarization evidence exists. |
| Product scope | Supported-method and refusal boundaries are published in the README and method contracts. | No universal recovery, fragmented-file, overwritten/TRIM, encrypted, or controller-discard claim is permitted. |

## Release prerequisites

| Area | Required evidence before a public release |
| --- | --- |
| **Source control** | Clean release-candidate tree; authorized commit identity; protected default branch; current branch and required checks match the contribution model. |
| **Local quality** | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`, `cargo test --workspace`, `cargo build --workspace`, and `sh scripts/verify-all.sh` pass on the intended release commit. |
| **Security** | `cargo audit` reports no known vulnerabilities; any maintenance or unsoundness advisory is reviewed and recorded; no credentials, real images, or private data are committed; the security reporting route is enabled. |
| **CI** | The exact release commit has green hosted workflow evidence for formatting, tests, fixture verification, build, desktop smoke where applicable, security checks, and all supported platform packaging. |
| **Desktop artifacts** | Clearly labeled platform/architecture artifacts are built, hashed, and tested on every platform that will be advertised. Windows installer acceptance and macOS package validation must be recorded before broad distribution claims. |
| **Documentation** | README, safety boundaries, architecture, method contracts, changelog, contribution guide, security policy, project status, support policy, and maintainer contacts match the release behavior. |
| **Governance** | Branch protection, required contexts, workflow permissions, dependency policy, and release ownership match the published contribution model. |
| **Release metadata** | A semantic version, annotated tag, release notes, artifact hashes, known limits, support status, SBOM/provenance where enabled, and signing status are prepared. |

## Local preflight

Run the local matrix from the workspace root:

```sh
sh scripts/verify-all.sh
```

Then inspect the working tree and release metadata. Verify that generated fixtures are deterministic, build output is excluded, and the changelog describes only behavior actually present in the intended commit. Confirm the package license metadata and `LICENSE` file agree.

A dependency audit vulnerability blocks a release. A maintenance or unsoundness advisory without an available compatible remediation must be reviewed before release, recorded with the affected dependency path and mitigation status, and revisited when the dependency graph changes. It must not be hidden by suppressing the `cargo audit` command or by claiming a clean advisory result.

When both a Linux bundle and a Linux-host Windows cross-target review ZIP have been locally verified, record their executed checks, hashes, byte sizes, environment, and intentional limitations with the [local release-evidence contract](local-release-evidence-v1.md). This record is useful for review, but it does not replace native platform testing, hosted workflows, governance, tags, or a public release record.

## Hosted workflow and governance

Push only an authorized verified commit. Hosted validation must run with least-privilege read-only permissions and should execute the same local quality command. Keep validation, artifact publication, dependency updates, and release publication separate, with only the permissions necessary for each action.

For DiskTrace, `main` requires the exact Linux `Rust workspace and recovery fixtures`, native Windows `Windows x86_64 bundle and installer`, `CodeQL Rust analysis`, and `macOS 14 ARM64 workspace validation` contexts. Pull requests must be current with `main` before merge and require one current CODEOWNERS review; linear history and resolved conversations are required; force pushes and branch deletion are blocked. The authorized owner direct-main maintenance path remains available. Do not describe a release as verified merely because a workflow configuration exists.

Record the release commit SHA and successful workflow URLs in the release record. If a check fails, fix the underlying cause, repeat local verification, and publish a new verified commit rather than rewriting shared history.

## Artifact expectations

Release artifacts must identify the target platform and architecture, include version information, and be accompanied by a SHA-256 checksum. Prefer reproducible build instructions and explicit platform support statements over a binary that cannot be traced to a source revision. Test application launch and a representative safe recovery workflow on each supported platform before advertising it.

Never package real user images, recovered files, private session manifests, credentials, or debug logs containing sensitive paths into a release artifact. Do not enable Windows Authenticode signing or macOS signing/notarization until the owner has explicitly authorized the credential strategy and public release scope.

## Release notes

Release notes must state what changed, which recovery methods were added or altered, source and destination safety behavior, fixture and verification evidence, migration impact, known intentional limits, advisory status, and the supported platform/artifact list. Do not claim that every deleted file is recoverable, that recovered content is complete, or that a free allocation bitmap proves byte preservation.

## Controlled release decision

After all release prerequisites are green on one exact target, prepare a draft decision package containing the commit SHA, local and hosted evidence, platform artifacts/checksums, release notes, advisory disposition, manual acceptance record, and known limits. Request explicit authorization before creating the annotated semantic tag, publishing a GitHub Release, uploading release assets, using signing credentials, or changing release visibility.

A release is complete only after the tag, release page, release assets, checksums, and exact target evidence all resolve to the intended revision.

## Post-release maintenance

Monitor the exact hosted workflow result, release page, artifact checksums, and security-reporting route. Correct a material release issue with a documented follow-up release rather than silently replacing published artifacts. Update the changelog and contract documents whenever a parser boundary or user-visible evidence claim changes.
