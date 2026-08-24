# DiskTrace SBOM and provenance contract v1

## Purpose and boundary

DiskTrace generates a reproducible **CycloneDX 1.5 JSON Software Bill of Materials (SBOM)** for review artifacts built from a clean, trusted repository revision. The SBOM exposes resolved Cargo dependency information and the associated lightweight provenance record makes the source commit, source timestamp, generator version, target scope, document count, and document checksums inspectable.

> An SBOM describes declared software components for the recorded build input. It does **not** establish that all vulnerabilities are absent, that a binary is safe, that a recovery outcome is forensically complete, or that an artifact is a signed public release.

| Output | Purpose | Explicit limit |
|---|---|---|
| `*.cdx.json` | One CycloneDX 1.5 JSON document per workspace package, generated for all Cargo targets. | Dependency description is not an executable-security guarantee or a license/compliance determination. |
| `SHA256SUMS` | SHA-256 checksums for the emitted SBOM documents. | A checksum is an integrity observation, not a code signature or release attestation. |
| `sbom-provenance.json` | Records the exact source commit, commit timestamp, generator name/version, SBOM format, all-target scope, and document count. | It is project-authored metadata, not a cryptographically signed provenance statement. |
| Hosted review artifact | Makes the generated SBOM set available beside the bounded Windows build review artifact. | It is a short-retention CI artifact, not a GitHub Release asset or public supported download. |

## Generator contract

`scripts/generate-sbom.sh` requires a clean tracked source revision with no untracked files. It archives the exact `HEAD` revision into a temporary directory, runs the official OWASP `cargo-cyclonedx` tool there, and copies only generated `*.cdx.json` documents into the requested output directory. This avoids writing generated SBOMs into the working tree and refuses a modified or untracked source state rather than silently describing an ambiguous input.

The generator is pinned to **`cargo-cyclonedx 0.5.9`**, which declares Rust 1.85.0 compatibility and therefore works with DiskTrace’s pinned Rust 1.97.1 toolchain.[1] It invokes `cargo cyclonedx --format json --all --target all --spec-version 1.5` with `SOURCE_DATE_EPOCH` set to the exact commit timestamp. The tool uses Cargo metadata and Cargo.lock information to describe the trusted Cargo project; it must not be run against untrusted source because its upstream documentation warns that Cargo operations may run arbitrary code.[2] DiskTrace runs it only against its checked-out repository revision in local verification experiments and GitHub-hosted CI.

| Generator decision | Rationale |
|---|---|
| Temporary archived source | Prevents generated SBOM files from contaminating the source worktree and fixes the input to one commit. |
| Clean/untracked refusal | Avoids publishing a document that ambiguously represents modified local source. |
| `cargo-cyclonedx 0.5.9` | Pins a known generator release compatible with the project toolchain. |
| CycloneDX 1.5 JSON | Produces machine-readable, per-package dependency documents in a recognized SBOM format. |
| `--all --target all` | Preserves resolved dependency information across Cargo targets rather than describing only the host target. |
| Commit-derived `SOURCE_DATE_EPOCH` | Makes the generator’s timestamp input deterministic for one source revision. |
| Document checksums | Makes review-artifact contents inspectable without confusing the result with code signing. |

## Hosted artifact policy

The Windows distribution workflow installs the pinned generator, runs the script after building the portable ZIP and installer, and uploads the resulting `dist/sbom` directory as part of the existing review artifact. The Windows workflow remains read-only with respect to repository contents and does not upload source images, recovered payloads, credentials, keys, case files, telemetry, or runtime-AI data.

This contract intentionally **does not create an attestation** for routine CI review artifacts. GitHub recommends attestations for software consumers are expected to run or for manifests that include detailed hashes, and advises against signing frequent automated-testing builds.[3] GitHub artifact attestation also requires additional `id-token: write` and `attestations: write` permissions.[4] DiskTrace will consider an attestation only in a separately authorized release decision, after identifying a consumer-facing immutable artifact, a verification procedure, a retention/lifecycle policy, and the exact least-privilege workflow change. No such authorization, tag, GitHub Release, release asset, signing identity, or notarization action is created by this contract.

## Review procedure

For an exact hosted workflow result, download the review artifact through the GitHub Actions interface, compare each `*.cdx.json` document against `SHA256SUMS`, inspect `sbom-provenance.json` for the expected commit and generator version, and cross-check the workflow URL/commit. Treat the review as supply-chain transparency evidence only. Dependency advisories continue to be evaluated through `cargo audit`, Dependabot, CodeQL, and human triage; the SBOM does not replace those controls.

## Acceptance criteria

| Criterion | Required evidence |
|---|---|
| Local static contract | `sh scripts/verify-sbom.sh` passes, including shell syntax and the pinned generator/source-boundary assertions. |
| Clean-source behavior | The generator refuses tracked or untracked local modifications. |
| Positive output behavior | On a clean exact revision, the generator emits one or more CycloneDX JSON documents, `SHA256SUMS`, and `sbom-provenance.json`. |
| Hosted execution | The exact Windows workflow completes the SBOM generation and uploads the review artifact without weakening existing bundle, installer, or audit checks. |
| Public claim boundary | Documentation names the exact workflow evidence and continues to state that review artifacts are not signed releases, attestations, or supported downloads. |

## References

[1]: https://crates.io/crates/cargo-cyclonedx/0.5.9 "cargo-cyclonedx 0.5.9 on crates.io"
[2]: https://github.com/cyclonedx/cyclonedx-rust-cargo "CycloneDX Rust Cargo plugin"
[3]: https://docs.github.com/en/actions/concepts/security/artifact-attestations "GitHub Docs: Artifact attestations"
[4]: https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations "GitHub Docs: Using artifact attestations to establish provenance"
