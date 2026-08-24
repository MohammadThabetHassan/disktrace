# DiskTrace: local release-readiness assessment

**Assessment date:** 2026-08-18  
**Scope:** Local workspace only. No Git repository was initialized, no commit was created, and nothing was pushed or published.

## Executive assessment

DiskTrace is now a **strong local pre-release implementation**. The primary desktop workflow is GUI-first, the recovery methods are intentionally bounded and explainable, evidence-session and receipt behavior are documented, and the full local verification matrix passes. The desktop dependency graph was upgraded to remove a known browser-launch vulnerability, and the quality gates are now executable locally and configured for future hosted CI.

The honest assessment is **92/100 for local implementation quality** and **76/100 for public-release readiness**. The difference is not a claim that the code is unreliable; it reflects evidence that cannot be created locally, including repository governance, hosted CI results, native Windows and macOS validation, public release metadata, and authorized code signing. This workspace should not yet be described as a public 10/10 release, but it is a credible, well-verified foundation for one.

| Dimension | Local score | Basis for the score |
|---|---:|---|
| Product value and guided UX | 92/100 | The desktop application is the primary entry point and provides guided recovery, evidence, sessions, filters, validation explanations, and safe export behavior. |
| Forensic safety and decision quality | 96/100 | Read-only source handling, dual hashes, destination policy, receipts, explicit validation labels, and refusal conditions are implemented and tested. |
| Engineering quality | 94/100 | The workspace passes format, strict Clippy, warning-free documentation, 71 unit tests, deterministic end-to-end fixtures, build, and desktop smoke checks. |
| Dependency security | 91/100 | `cargo audit` reports no known vulnerabilities. One reviewed unmaintained GUI-text-rendering dependency remains and is tracked transparently. |
| Documentation and governance design | 91/100 | Public-facing safety, architecture, contribution, security, conduct, release, distribution, and advisory documentation is present; hosted governance has not yet been activated. |
| Distribution readiness | 78/100 | A verified Linux x86_64 GUI bundle exists. Windows bundle and installer automation are configured but not executed natively; macOS packaging remains unimplemented. |

## Completed in this increment

The desktop stack now uses **eframe 0.33.3** and resolves `webbrowser` to 1.2.4. This removes the previous `webbrowser` 0.8.15 path that failed the RustSec scan for RUSTSEC-2026-0257. The advisory identifies Unix `BROWSER` template handling as vulnerable through webbrowser 1.2.1 and specifies 1.2.2 or later as the fixed line.[1]

The desktop configuration now selects eframe’s OpenGL renderer explicitly while retaining the native accessibility, bundled-font, Wayland, and X11 features required by the application. DiskTrace has no custom WGPU rendering requirement, so this also removes an otherwise unused macOS WGPU dependency path to the unmaintained `paste` crate. The current scan retains one non-vulnerability maintenance advisory for `ttf-parser` 0.25.1, brought in by eframe’s text-rendering stack. RustSec classifies it as unmaintained, provides no patched version, and identifies `skrifa` as a potential alternative.[2] The exact path, disposition, and review trigger are recorded in [`dependency-advisories.md`](dependency-advisories.md).

The release matrix now performs format checking, strict Clippy, warning-free workspace documentation generation, RustSec auditing, unit and doc tests, all deterministic recovery fixtures, full workspace build, and an Xvfb desktop smoke launch. The pinned toolchain declares both `rustfmt` and `clippy`. Future Linux CI installs the locally validated `cargo-audit` 0.22.2 before invoking the same matrix, while the Windows workflow adds strict Clippy before packaging its native artifacts.

| Verification item | Result | Evidence |
|---|---|---|
| Formatting | Pass | `cargo fmt --all -- --check` passed through `scripts/verify-all.sh`. |
| Strict linting | Pass | `cargo clippy --workspace --all-targets -- -D warnings` passed. |
| API documentation | Pass | `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` passed. |
| Dependency scan | Pass with one reviewed maintenance warning | `cargo audit` reported zero known vulnerabilities and RUSTSEC-2026-0192 for `ttf-parser` 0.25.1. |
| Tests | Pass | All 71 workspace unit tests and doc tests passed. |
| Recovery fixtures | Pass | Foundation, FAT12/FAT16, sessions, PDF/ZIP/Open XML, exFAT, NTFS resident, and NTFS contiguous verifiers passed. |
| Native desktop smoke | Pass | The local desktop application launched successfully under Xvfb. |
| Linux bundle verification | Pass | Archive checksum, staged-file checksums, CLI help, required documentation, and GUI launcher smoke check passed. |

## Verified local artifact

| Artifact | Status | SHA-256 |
|---|---|---|
| `dist/DiskTrace-0.1.0-linux-x86_64.tar.gz` | Rebuilt and verified locally; includes desktop launcher, CLI, release process, and advisory register | `21fadca5b67d1deacde3c16787583837b3f72086df6f2f52776228b5954482b8` |

The archive is approximately 5.9 MB. It is a **local Linux x86_64 artifact**, not a public or signed release. Its GUI launcher is the primary entry point and its embedded checksums verified successfully.

## Remaining gates to reach a public 10/10 release

The following items are external release gates, not items that can be honestly completed inside this local-only workspace. First, create the repository only when an authorized commit identity is available; then add the current local work as reviewed commits, publish the Apache-2.0 source, and enable branch protection, required checks, private security reporting, and a public issue tracker.

Second, obtain hosted evidence for the exact release commit. The configured Linux and Windows workflows must run successfully on the hosting provider, and their URLs and commit SHA must be recorded in release notes. Windows requires native build, portable bundle verification, installer install/uninstall validation, and GUI smoke evidence. macOS still needs a native build, application packaging, GUI smoke validation, and, before broad distribution, an authorized code-signing and notarization process.

Third, complete manual release acceptance. This should include keyboard-only workflow testing, assistive-technology testing on supported platforms, representative recovery-image testing outside synthetic fixtures, installer UX review, and a security review of the actual released artifact. The current `ttf-parser` maintenance advisory must be rechecked at every eframe upgrade and becomes a release blocker if it is reclassified as a vulnerability.

Finally, keep the product boundary explicit. FAT32, ext-family, APFS, fragmented-file reconstruction, expanded NTFS runlist handling, and macOS-native distribution are valuable roadmap directions, but they must be delivered as separately tested capabilities rather than implied by the current supported-method list.

## Conclusion

The local implementation has moved materially closer to release-grade quality. It now has a clean known-vulnerability scan, strict reproducible quality gates, a reviewed advisory register, aligned local and future CI checks, and a freshly verified Linux GUI bundle. The next appropriate action is **not** to claim public release readiness; it is to create the repository and run the remaining external validation gates under the user’s authorized identity.

## References

[1]: https://rustsec.org/advisories/RUSTSEC-2026-0257.html "RUSTSEC-2026-0257: Unix BROWSER handling allows browser argument injection"
[2]: https://rustsec.org/advisories/RUSTSEC-2026-0192.html "RUSTSEC-2026-0192: ttf-parser is unmaintained"
