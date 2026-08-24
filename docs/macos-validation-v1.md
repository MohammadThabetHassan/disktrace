# DiskTrace macOS validation v1

## Purpose and evidence boundary

This contract defines the hosted macOS validation target for DiskTrace. It is designed to replace the prior absence of macOS build evidence with a reproducible Apple-silicon workspace check and a review-only unsigned binary. It does **not** establish universal macOS compatibility, Intel support, app-bundle behavior, installer quality, signing, notarization, Gatekeeper acceptance, manual desktop usability, or a production release.

> A passing macOS workflow means the recorded revision compiled, linted, tested, and produced the named unsigned ARM64 desktop binary on the stated hosted runner. It does not make that binary safe to distribute, trusted by macOS, or suitable for use on another operating-system version or processor architecture.

| Evidence area | Hosted control | What it establishes | What it does not establish |
|---|---|---|---|
| Target platform | GitHub-hosted `macos-14` runner | macOS 14 Apple-silicon/ARM64 build and test evidence for one exact source revision. | Intel/x86_64 compatibility, all macOS releases, or physical-device diversity. |
| Workspace quality | Formatting, strict Clippy, and `cargo test --workspace` | The workspace passes the stated Rust checks on the recorded target. | A GUI launch, accessibility review, or real-user workflow result. |
| Desktop build | `cargo build --release -p ef-desktop --bin evidenceforge-desktop` | The named desktop binary is produced for the runner target. | A `.app` bundle, disk image, installer, signing, or notarization. |
| Architecture and integrity | `file` confirms `arm64`; `shasum -a 256` records a hash | The review artifact has the expected runner architecture and a recorded checksum. | Artifact provenance beyond the workflow, code-signature trust, or release distribution approval. |
| Review artifact | A short-retention Actions artifact contains the raw executable and checksum. | Maintainers can inspect the exact workflow output. | A GitHub Release asset, public supported download, or a safe executable payload. |

## Runner selection

GitHub documents `macos-14` as an Apple-silicon M1 hosted runner and lists ARM64 macOS runners among the standard public-repository targets.[1] This workflow deliberately names that runner rather than using an ambiguous `macos-latest` label. The resulting platform statement must therefore remain **macOS 14 ARM64 hosted validation**, not “macOS supported.” GitHub also documents limits specific to ARM64 macOS runners, including the absence of a static UUID/UDID; this workflow does not use Apple signing credentials or provisioning profiles.[1]

The project does not currently run an Intel macOS workflow. GitHub’s retirement guidance identifies current Intel labels separately from the ARM64 labels, so an ARM64 pass must never be generalized to Intel Macs.[2]

## Workflow contract

The workflow is located at `.github/workflows/macos-verify.yml` and runs on pushes to `main` and pull requests targeting `main`. It uses read-only repository contents permission, an explicit concurrency group, and a 30-minute job timeout. It installs the project’s pinned Rust 1.97.1 toolchain, runs formatting, strict Clippy, and workspace tests, then builds only the declared desktop binary in release mode.

After building, the workflow verifies that the expected executable exists, verifies its ARM64 identity with the macOS `file` utility, creates a SHA-256 checksum with `shasum -a 256`, and uploads both items as a 14-day review artifact. No source image, recovered payload, telemetry, runtime AI, user account, code-signing identity, Apple credential, notarization token, or release publication step is introduced.

| Workflow input or output | Boundary |
|---|---|
| Input | Public repository source at the exact triggering commit. |
| Build output | Unsigned `evidenceforge-desktop` ARM64 executable for review only. |
| Artifact retention | 14 days in GitHub Actions; not a semantic-versioned release asset. |
| Permissions | `contents: read` only; no package publication, release upload, or credential access. |
| Cancellation | Newer runs for the same workflow/ref cancel older in-progress runs; use the completed exact SHA for evidence. |

## Acceptance and promotion conditions

A green workflow should be recorded with its exact source SHA, Actions URL, runner name, and artifact checksum only after the run completes. A failed workflow is a non-passing result and must be corrected without weakening the build, architecture, or source-safety objective. A cancelled workflow is not validation evidence.

Before advertising a macOS distribution, maintainers must separately define and validate the target architecture or architectures, create and test the appropriate package or app bundle, perform a manual desktop-acceptance record on the exact target, establish authorized signing/notarization procedures, document Gatekeeper behavior, produce artifact checksums and provenance, and obtain explicit authorization for any tag, GitHub Release, release asset, signing credential, notarization, or production-release claim.

## References

[1]: https://docs.github.com/en/actions/reference/runners/github-hosted-runners "GitHub Docs: GitHub-hosted runners"
[2]: https://github.blog/changelog/2025-09-19-github-actions-macos-13-runner-image-is-closing-down/ "GitHub Changelog: macOS Intel and ARM64 runner migration guidance"
