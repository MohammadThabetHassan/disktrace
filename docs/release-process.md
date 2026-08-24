# Release process

This document defines the intended release gate for DiskTrace. The current workspace is local and uncommitted, so none of the hosted checks or publication steps below have been completed. A local archive is not a public release.

## Release prerequisites

| Area | Required evidence before a public release |
|---|---|
| **Source control** | Repository created with an authorized owner, clear default branch, reviewed working tree, and only authorized commit identity. |
| **Local quality** | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`, `cargo test --workspace`, `cargo build --workspace`, and `sh scripts/verify-all.sh` pass on the intended release commit. |
| **Security** | `cargo audit` reports no known vulnerabilities; any non-vulnerability advisories are reviewed and recorded; no credentials, real images, or private data are committed; security reporting route is enabled. |
| **CI** | The exact release commit has green hosted workflow evidence for formatting, tests, fixture verification, build, and applicable security checks. |
| **Desktop artifacts** | Installers or clearly labeled platform binaries are built, hashed, and tested on each supported platform. |
| **Documentation** | README, safety boundaries, architecture, contracts, changelog, contribution guide, security policy, license, and maintainer contacts match the release behavior. |
| **Governance** | Branch protection and required checks match the contribution model; force pushes and branch deletion are prohibited where the host permits it. |
| **Release metadata** | A semantic version, annotated tag, release notes, artifact hashes, known limits, and support status are prepared. |

## Local preflight

Run the local matrix from the workspace root:

```sh
sh scripts/verify-all.sh
```

Then inspect the working tree and release metadata. Verify that generated fixtures are deterministic, build output is excluded, and the changelog describes only behavior actually present in the intended commit. Confirm the package license metadata and `LICENSE` file agree.

A dependency audit vulnerability blocks a release. A maintenance or unsoundness advisory without an available compatible remediation must be reviewed before release, recorded with the affected dependency path and mitigation status, and revisited when the dependency graph changes. It must not be hidden by suppressing the audit command or by claiming a clean advisory result.

When both a Linux bundle and a Linux-host Windows cross-target review ZIP have been locally verified, record their executed checks, hashes, byte sizes, environment, and intentional limitations with the [local release-evidence contract](local-release-evidence-v1.md). This record is useful for review, but it does not replace native platform testing, hosted workflows, governance, tags, or a public release record.

## Hosted workflow and governance

After a repository exists, push only an authorized verified commit. The hosted validation workflow should run with least-privilege read-only permissions and should execute the same local quality command. Configure any deployment or artifact-publishing workflow separately from validation, with only the permissions necessary for that action.

Do not require checks that the repository does not produce, and do not label a release as verified merely because a workflow configuration exists. Record the commit SHA and successful workflow URLs in the release record. If a check fails, fix the underlying cause, repeat local verification, and publish a new verified commit rather than rewriting shared history.

## Artifact expectations

Release artifacts must identify the target platform and architecture, include version information, and be accompanied by a SHA-256 checksum. Prefer reproducible build instructions and explicit platform support statements over a binary that cannot be traced to a source revision. Test application launch and a representative safe recovery workflow on each supported platform before advertising it.

Never package real user images, recovered files, private session manifests, credentials, or debug logs containing sensitive paths into a release artifact.

## Release notes

Release notes should state what changed, which recovery methods were added or altered, source and destination safety behavior, fixture and verification evidence, migration impact, known intentional limits, and the supported platform/artifact list. Do not claim that every deleted file is recoverable, that recovered content is complete, or that a free allocation bitmap proves byte preservation.

## Post-release maintenance

Monitor the exact hosted workflow result, release page, artifact checksums, and security-reporting route. Correct a material release issue with a documented follow-up release rather than silently replacing published artifacts. Update the changelog and contract documents whenever a parser boundary or user-visible evidence claim changes.
