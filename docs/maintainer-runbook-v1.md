# DiskTrace maintainer runbook v1

## Purpose and operating boundary

This runbook defines how the current maintainer handles public-source changes, hosted verification, dependency updates, security reports, and any future release decision. It is an operational contract for the repository's current pre-release posture, not a promise that a particular recovery method works for every device, format, or data-loss event.

> DiskTrace remains a local, source-read-only forensic workflow. It does not upload sources, write to a selected source, execute recovered content, or claim universal, fragmented, overwritten, or TRIM recovery.

| Area | Current operating rule | Evidence or escalation point |
|---|---|---|
| Repository ownership | `MohammadThabetHassan` is the current authorized maintainer and CODEOWNERS reviewer. | [`.github/CODEOWNERS`](../.github/CODEOWNERS) |
| Protected `main` | Pull requests require a current owner review, resolved conversations, linear history, and green required hosted checks. The owner’s currently authorized direct-main maintenance path remains available because administrator enforcement is intentionally disabled. | Repository branch-protection settings |
| CI evidence | Treat the exact commit SHA as the unit of evidence. A green earlier run does not approve a newer revision. | [Release process](release-process.md) |
| Recovery scope | Add or advertise a method only after a bounded parser contract, deterministic positive fixtures, deterministic refusal fixtures, and local plus hosted verification. | [Project status](project-status.md) |
| User privacy | Never ask contributors to place real disk images, recovered payloads, credentials, keys, or personal data in a public issue. | [Security policy](../SECURITY.md) |

## Change intake and review

A proposed change begins with a minimal reproduction, an explicit expected result, and the affected platform or fixture. A security concern follows the private route in the security policy rather than a public issue. Feature requests that expand recovery must explain the evidence boundary and include both a successful case and a refusal case; a request alone must never become a public support claim.

| Change type | Required maintainer response before merge or direct maintenance | Stop condition |
|---|---|---|
| Source or UI behavior | Run the full local verification matrix, inspect the focused diff, and ensure the user-visible documentation remains accurate. | A lint, test, fixture, smoke, packaging, or documentation gate fails. |
| Recovery-method expansion | Review format geometry, allocation/fragmentation assumptions, source safety, ambiguity handling, fixtures, cancellation, and false-positive controls. | The method would infer unsupported extent, allocation, or byte-preservation evidence. |
| Dependency update | Review the dependency range, changelog or advisory context, lockfile delta, compatibility impact, and fresh verification. Do not merge a stale bot pull request by default. | It is a major update, lacks current evidence, changes policy-relevant behavior, or cannot be reproduced in an authorized maintainer commit. |
| Workflow or supply-chain change | Check least-privilege permissions, pinned or reviewed action references, artifact flow, and whether generated evidence is truthful. | The change introduces unreviewed credentials, a broader token permission, opaque artifact provenance, or an unsupported platform claim. |
| Documentation-only change | Run `sh scripts/verify-release-docs.sh` and check every status statement against its exact evidence. | It claims an untested platform, release, signature, certification, or recovery capability. |

## CI triage and exact-evidence handling

When a required workflow fails, reproduce the relevant command locally where possible, retain the failing log or concise cause, and make the narrowest safe correction. A hosted failure caused by a missing platform library, packaging environment, or test dependency is not a reason to weaken the smoke test or remove a safety check. The corrected commit must receive a new full local matrix and new exact-SHA hosted evidence.

| Symptom | First response | Required disposition |
|---|---|---|
| Format, lint, rustdoc, or workspace-test failure | Reproduce the named local command and inspect the smallest affected module or fixture. | Correct the underlying defect; do not suppress warnings or remove coverage merely to pass. |
| Fixture mismatch | Confirm the fixture's expected range, identity, refusal rationale, and parser boundary. | Preserve or add deterministic positive and refusal coverage before changing expected output. |
| Linux desktop smoke failure | Diagnose package/runtime availability while retaining the native smoke objective. | Document the platform dependency and retest on the exact commit. |
| Windows bundle or installer failure | Validate the native packaging script, installer configuration, checksum inputs, and verification command. | Publish no Windows support expansion until the native workflow is green for the exact revision. |
| `cargo audit` finding | Identify whether the advisory is reachable, patched, or a justified documented exception. | Prefer an available safe update; maintain a precise advisory record if an exception is necessary. |

## Dependency and automation posture

Dependabot is configured to group routine patch and minor updates and to keep a small review queue. It is an input to maintainer review, not an auto-merge authority. Before accepting an update, bring it to a current base, assess compatibility, run the full local matrix, and obtain fresh hosted evidence. If preserving the sole-author history matters for a change, reproduce the reviewed update in a new maintainer-authored commit rather than merging a bot-authored commit.

No workflow may use unattended source-image uploads, cloud recovery services, runtime AI recovery, telemetry, or unreviewed secret access. Any future automated release, provenance, signing, or notarization work must be introduced with least privilege and documented credential boundaries.

## Incident, disclosure, and withdrawal procedure

Potential vulnerabilities, accidental sensitive-data exposure, artifact substitution, false recovery claims, or source-safety defects require prompt containment. Do not request or redistribute sensitive material while diagnosing an issue. Follow the private reporting channel in the security policy for vulnerabilities, limit public discussion to safe metadata, and record the remediation in the applicable changelog, contract, or advisory record after disclosure can occur safely.

If a future public artifact is found to be materially defective, stop advertising the affected version, preserve the original evidence trail, state the known scope and safe workaround, and issue a documented follow-up release only after corrected exact-SHA evidence is available. Never silently replace a published artifact or checksum. Creating tags, GitHub Releases, release assets, signatures, notarizations, or production-release claims remains separately authorization-gated.

## Maintainer review record

For a material change, record the following in the pull request, commit-associated notes, or release evidence package. This is a factual checklist, not a substitute for the underlying logs.

| Record item | Required value |
|---|---|
| Exact commit | Full SHA reviewed or released |
| Scope | Behavioral change and explicit non-claims |
| Local evidence | Commands run and result |
| Hosted evidence | Exact workflow URLs and result, where applicable |
| Platform boundary | Platforms actually validated and those still unvalidated |
| Privacy review | Confirmation that no source image, recovered payload, credential, key, or personal data was introduced |
| Follow-up | Open limitation, refusal fixture, or authorized next action |

## Related contracts

Read this runbook together with the [contribution guide](../CONTRIBUTING.md), [security policy](../SECURITY.md), [project status](project-status.md), [release process](release-process.md), [release scorecard](release-scorecard-v1.md), and the recovery-method contracts. If they disagree, the more conservative source-safety or release boundary governs until the documents are reconciled and verified.
