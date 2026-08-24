# DiskTrace desktop acceptance v1

## Purpose and evidence boundary

This checklist records manual desktop acceptance for the local, read-only forensic workflow. It complements—not replaces—deterministic workspace tests, recovery fixtures, package verification, and hosted CI. A completed record applies only to the stated application commit, operating system, package or build, display conditions, and test controls. It is **not** an accessibility certification, a universal usability claim, a data-recovery guarantee, or evidence for an untested platform.

> Do not use a real victim disk image, recovered payload, personal data, credential, cryptographic key, or executable content for this checklist. Use the repository's synthetic, minimized, deterministic controls and an empty or disposable export destination.

| Evidence type | What it can establish | What it cannot establish |
|---|---|---|
| Automated local and hosted verification | Deterministic code, fixture, build, package, and scripted smoke behavior for the exact revision. | Real-user interaction quality, keyboard discoverability, assistive-technology compatibility, or native validation on a platform not tested. |
| This manual checklist | A reviewer observed the listed safe workflow on the recorded target. | A certification, exhaustive accessibility audit, recovery of arbitrary files, or support for other target versions. |
| Recovery receipt and case brief | The app recorded its defined identity, range, destination, and audit metadata. | Independent proof that recovered content is complete, safe to execute, or valid for every evidentiary purpose. |

## Preconditions

Before beginning, record the exact commit SHA, application version or build identifier, operating system and version, architecture, display scale, package or launch path, keyboard layout, and fixture/control names. Confirm that the source is opened read-only by the application contract, that the selected export destination is separate from the source, and that test controls contain no sensitive material.

A reviewer must stop the session and record a failure if the application offers to write to the selected source, accepts the source directory as an export destination, attempts to open or execute previewed recovered content, implies universal recovery, or makes a support claim beyond the recorded platform evidence.

## Manual acceptance checklist

| ID | Scenario | Expected safe result | Result and evidence to record |
|---|---|---|---|
| DA-01 | Launch the desktop application from the recorded build or package. | The start screen identifies a local recovery session and does not request account, cloud, telemetry, or runtime-AI access. | Pass, fail, or blocked; screenshot only if it contains no sensitive material. |
| DA-02 | Complete the primary recovery workflow with keyboard navigation only, including focus movement, activation, cancellation, and return to the workspace. | Focus order is visible and usable; essential actions are reachable without a mouse; labels and error text remain understandable. | Keys attempted, focus issue if any, and observed action. |
| DA-03 | Select a supported synthetic source and start a scan. | The source is treated as read-only, method boundaries are visible, progress or cancellation state is intelligible, and no unsupported universal-recovery language appears. | Fixture/control name, method, candidate count or refusal result, and observed status. |
| DA-04 | Trigger or simulate a source-changed/identity mismatch state using the repository's safe control. | The application clearly identifies the mismatch and blocks identity-bound actions until the source is rechecked. | Exact control, visible message, and whether protected action was unavailable. |
| DA-05 | Select a candidate preview, including a controlled preview failure when available. | Preview is range-bound to the verified source identity and never executes or renders opened recovered content; a failure is explained without exposing unsafe content. | Candidate/control, status shown, and action availability. |
| DA-06 | Attempt export with a destination that violates the separate-destination policy, then choose a separate disposable destination. | The unsafe destination is refused with a useful explanation; the separate destination requires deliberate confirmation and receives only the selected output. | Both destination cases, confirmation text, and created output location. |
| DA-07 | Inspect export audit data and save a case brief after the safe export. | Receipts and case brief contain the defined provenance/audit fields and no unexpected source writes or cloud transfer occur. | Paths, identity/result fields observed, and redacted-safe record reference. |
| DA-08 | Exercise a clear failure or cancellation path during scanning or export. | Cancellation or failure leaves the source untouched, communicates what happened, and does not claim recovery success. | Trigger, final status, and destination cleanup observation. |
| DA-09 | Review visible text at the recorded display scale and a reduced window size. | Essential actions remain discoverable, text is not materially clipped, contrast and hierarchy remain practical for the exercised workflow, and the safe boundaries are visible. | Window/display conditions and any clipping, ambiguity, or contrast concern. |
| DA-10 | Close and reopen only through the documented safe session path. | Persisted session data is handled according to the session contract, unknown or invalid session data is rejected safely, and the reopened state does not bypass source identity checks. | Session control, observed result, and any refusal message. |

## Acceptance record template

Use one record per platform and exact application revision. A `blocked` outcome is evidence of a missing prerequisite, not a passing result. Preserve only safe metadata and redacted screenshots.

| Field | Record |
|---|---|
| Reviewer | |
| Date and time zone | |
| Commit SHA | |
| Application build, package, or launch command | |
| Operating system, version, and architecture | |
| Display scale, resolution, and keyboard layout | |
| Synthetic controls/fixtures used | |
| DA-01 through DA-10 results | |
| Defects or accessibility observations | |
| Exact local/hosted verification evidence associated with the commit | |
| Scope statement | This record covers only the target stated above; it does not certify accessibility or advertise untested platforms. |

## Current status

No completed manual record is asserted by this document. The existing public evidence covers hosted Linux verification and a hosted native Windows bundle/installer workflow for their recorded revisions. It does **not** establish macOS validation, a manual Windows install/uninstall acceptance result, cross-platform signing or notarization, accessibility certification, or a production release. Add a dated record only after the checklist is performed on the exact target and its evidence is safely retained.

## Related contracts

This checklist should be used with the [GUI workflow](gui-workflow-v1.md), [session persistence contract](session-persistence-v1.md), [export and audit verification](export-audit-v1.md), [project status](project-status.md), and [release process](release-process.md). The most conservative applicable safety boundary governs.
