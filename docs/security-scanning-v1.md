# DiskTrace security scanning v1

## Purpose and scope

DiskTrace uses complementary automated controls to identify selected code and dependency risks before they become release evidence. The controls improve reviewability; they do **not** establish that the application is secure, free of defects, certified, safe to execute recovered content, or appropriate for every forensic workflow.

> A successful scan means that the configured tool completed for the recorded commit and configuration. It does not prove the absence of all vulnerabilities, unsafe recovery outcomes, malicious recovered payloads, or operational misuse.

| Control | Repository mechanism | Intended contribution | Explicit limit |
|---|---|---|---|
| Rust code scanning | `.github/workflows/codeql.yml` runs CodeQL’s `security-extended` Rust queries. | Highlights supported static-analysis findings in the repository’s code. | Static analysis is incomplete by design and a finding requires maintainer triage. |
| Dependency advisory review | `cargo audit` runs in the local matrix and hosted Linux verification; Dependabot security updates are enabled. | Identifies published advisory information associated with dependency resolution. | It does not prove every dependency or transitive behavior is safe. |
| Source-safety regressions | Deterministic workspace tests and method-specific refusal fixtures run in `sh scripts/verify-all.sh`. | Detects defined source-read-only, identity, range, export, and recovery-contract regressions. | It does not establish legal admissibility, recovery completeness, or support for untested formats. |
| Vulnerability handling | `SECURITY.md` defines the private reporting route. | Provides a safe escalation path without public disclosure of sensitive case data. | It does not replace a response plan tailored to a disclosed incident. |

## CodeQL workflow contract

The CodeQL workflow runs on pushes to `main`, pull requests targeting `main`, and a weekly scheduled scan. GitHub documents these event types as the normal advanced-setup pattern for current changes, pull-request feedback, and newly discovered issues in an otherwise unchanged default branch.[1] Rust is a supported CodeQL language, and the configured identifier is `rust`.[1] GitHub announced general availability for Rust CodeQL scanning in October 2025.[2]

The workflow uses an Ubuntu 24.04 runner, checks out the repository, initializes CodeQL for Rust with `security-extended` queries in its supported build-free mode, and submits the analysis category `/language:rust`. The first hosted attempt established that the current Rust extractor rejects manual build mode; the workflow therefore uses `build-mode: none`, which GitHub documents as supported for Rust.[3] The repository’s independent Linux verification workflow continues to compile and test the workspace with the pinned Rust toolchain; that build is complementary evidence, not an input CodeQL observes in this workflow.

| Workflow property | Deliberate choice | Rationale |
|---|---|---|
| Trigger scope | `main`, pull requests to `main`, and weekly schedule | Covers current changes, pull-request review, and later query/advisory knowledge without a source change. |
| Language and query suite | Rust with `security-extended` | Covers the project’s primary implementation language and uses a broader supported security query suite. |
| Build mode | Rust `none` (build-free) | Uses the current extractor’s supported Rust mode; independent CI still compiles the workspace. |
| Runner | `ubuntu-24.04` | Matches the established hosted Linux verification platform. |
| Permissions | `contents: read` and `security-events: write` only | Limits repository-content access while allowing CodeQL to submit security-analysis results. |
| Secrets and data handling | No custom secrets, uploads of source images, recovered payloads, telemetry, or runtime AI | Preserves the project’s local-first and privacy-first operating boundary. |

## Result triage and evidence handling

Treat a CodeQL alert as a security-review input, not an automatic proof of exploitability or a reason to suppress the result. First identify the exact commit, rule, code path, preconditions, and whether the alert is reachable in the supported local forensic workflow. Do not place real disk images, recovered content, credentials, keys, or private case material in alerts, issues, logs, or artifacts. A potential vulnerability must follow the private reporting route in `SECURITY.md`.

| Outcome | Maintainer action | Evidence rule |
|---|---|---|
| Confirmed or plausibly exploitable issue | Contain exposure, assess source-safety/privacy impact, correct it, and use the private disclosure path. | Preserve safe metadata, fix reference, tests, and exact scan evidence; do not disclose sensitive reproductions. |
| False positive or out-of-scope alert | Record the narrow technical rationale and reassess when code or query behavior changes. | Do not silently dismiss alerts or weaken scanning merely to improve a dashboard. |
| Tooling failure | Diagnose workflow action, toolchain, runner, build, or permission compatibility while preserving the analysis objective. | A failed scan is not a passing security result and must not be described as such. Recheck the current GitHub-supported language/build-mode combination before changing the workflow. |
| Clean completed scan | Record the exact commit, workflow URL, configuration, and date if used as release evidence. | Do not convert a clean scan into a security certification or a production-release claim. |

## Relationship to release decisions

CodeQL results, `cargo audit`, deterministic source-safety tests, and manual review are prerequisites for a stronger release decision, not substitutes for macOS evidence, manual desktop acceptance, installer acceptance, signing, notarization, provenance, an authorized tag, or an authorized GitHub Release. Any public status statement must point to the exact commit and completed workflow result it describes.

## References

[1]: https://docs.github.com/en/code-security/reference/code-scanning/workflow-configuration-options "GitHub Docs: Workflow configuration options for code scanning"
[2]: https://github.blog/changelog/2025-10-14-codeql-scanning-rust-and-c-c-without-builds-is-now-generally-available/ "GitHub Changelog: Rust CodeQL general availability"
[3]: https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/manage-your-configuration/codeql-for-compiled-languages "GitHub Docs: CodeQL code scanning for compiled languages"
