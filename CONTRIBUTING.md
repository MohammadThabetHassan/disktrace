# Contributing to DiskTrace

Thank you for improving DiskTrace. The project values **conservative recovery behavior, explainable evidence, deterministic tests, and privacy-first local workflows** over broad unsupported claims.

## Before contributing

Please read the [README](README.md), [Safety and evidence boundaries](docs/safety-and-evidence.md), [Architecture](docs/architecture.md), and the versioned contract relevant to the recovery method you are changing. A parser or carver must make its acceptance and refusal rules clear before it is presented as a recovery method.

Do not submit real disk images, private recovered material, credentials, keys, personal data, malware samples, or exploit payloads. Use a minimized synthetic fixture that contains only benign material and clearly documents its expected source offsets and output bytes.

## Local setup

Install a Rust toolchain compatible with Rust 2021 edition and clone the repository once it is published. From the workspace root, run:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo build --workspace
sh scripts/verify-all.sh
```

The full verification script runs the deterministic recovery matrix. On Linux it performs a native desktop smoke launch when `xvfb-run` is available. If your platform does not provide that command, run the formatter, tests, build, and relevant fixture verifiers; report the skipped desktop smoke step clearly.

## Recovery-method changes

A new recovery method is accepted only when it includes all of the following:

| Requirement | Purpose |
|---|---|
| A versioned contract in `docs/` | States supported inputs, structural checks, output semantics, and explicit refusals. |
| A distinct `RecoveryMethod` value | Keeps candidate, receipt, session, CLI, catalogue, and desktop provenance unambiguous. |
| Bounded parsing and overflow checks | Prevents untrusted image data from becoming unchecked offsets or allocations. |
| Positive, malformed, and reuse/overwrite controls | Demonstrates both accepted evidence and deliberate refusal behavior. |
| Deterministic fixture generator and expected artifacts | Makes a scenario reproducible without user data. |
| Shared-workflow test | Ensures the CLI and desktop use the same discovery and recovery behavior. |
| Catalogue explanation and desktop treatment | Explains the method’s evidence boundary in ordinary language. |
| End-to-end verifier | Exercises scan, filtering, safe export, receipt, and saved-session behavior where applicable. |

Never silently broaden an existing recovery method. If a capability is materially different, add a new method name and a new explanation. Do not label a candidate as intact, validated, or complete unless the implemented checks support that exact claim.

## Desktop changes

Keep the desktop usable without networking, cloud services, telemetry, account creation, or AI analysis. Preserve the background-scan cancellation behavior. Avoid rendering arbitrary recovered binary content; use metadata-only treatment unless a dedicated parser and safety model justify a preview.

When changing user-visible text, retain the distinction between a structural check, a recovery export, a free allocation bitmap, and a guarantee of byte preservation. The evidence view must never hide a known recovery limit.

## Tests and fixtures

Write fixture generators in `scripts/` and generated artifacts in `fixtures/<scenario-id>/`. A fixture directory should include at least the source image, expected recovered bytes when appropriate, and a small manifest describing the scenario and source range.

Keep fixtures small, deterministic, benign, and portable. Do not depend on a mounted device, an internet connection, random data, current time, or a user-specific path. Avoid tests that require a GUI display except the existing smoke launch wrapper.

## Pull requests and review

Until the project is published, all work remains local. After publication, propose focused changes with a concise description of the recovery or safety boundary, affected methods, test evidence, and intentional limitations. Keep unrelated formatting and generated build output out of a change.

A reviewer should be able to answer these questions from the change itself:

1. What new evidence is accepted or refused?
2. Which bytes can be exported, and why are their bounds safe?
3. What user-facing explanation changes?
4. Which deterministic positive and negative controls passed?
5. Does the change affect source integrity, destination policy, receipts, or session compatibility?

## Contributor licensing

By intentionally submitting a contribution for inclusion in DiskTrace, you agree that it may be distributed under the [Apache License 2.0](LICENSE). Do not submit code or assets whose license, ownership, or provenance you cannot confirm.

## Security concerns

Do not open a public issue for a suspected vulnerability or a privacy-sensitive parser issue. Follow [SECURITY.md](SECURITY.md) instead.
