# Release-candidate manual-acceptance kit v1

> **Preparation-only operator aid.** This kit helps a reviewer perform and record the existing desktop acceptance checklist. It does not create a release record, pass DA-01 through DA-10, authorize publication, replace a native-platform observation, or permit a tag, GitHub Release, public asset, signing, notarization, or production claim. Automated companions and the kit are not manual acceptance.

## Purpose and first-release scope

Use this kit only after a clean candidate commit is selected and all required local and hosted automated evidence is green for that exact commit. The intended first consumer-artifact scope is **Linux x86_64 and Windows x86_64 only**. macOS remains excluded from consumer distribution unless its separate package, architecture, manual-acceptance, signing/notarization, and Gatekeeper evidence requirements are completed.

> A hosted Windows runner can establish the defined build, bundle, installer, installed-file-integrity, and uninstall mechanics. It cannot establish visible GUI behavior, Start Menu or launcher usability, keyboard discoverability, accessibility, SmartScreen behavior, or a human acceptance result. Those observations require a real Windows desktop reviewer.

## Fail-closed preflight

Before opening DiskTrace, the reviewer records every item below. If a required item is unavailable, mark the platform record **blocked** and do not infer a pass from another operating system or CI workflow.

| Required record | Linux x86_64 | Windows x86_64 |
| --- | --- | --- |
| Exact candidate commit | Full clean protected-`main` SHA. | Full clean protected-`main` SHA. |
| Automated evidence | Exact-SHA local matrix and hosted Linux workflow URLs. | Exact-SHA local matrix and hosted native Windows workflow URLs. |
| Native environment | Recorded distribution/version, x86_64 architecture, desktop session, display scale/resolution, and keyboard layout. | Recorded Windows version/build, x86_64 architecture, display scale/resolution, and keyboard layout. |
| Candidate package | Verified Linux bundle and its checksum. | Verified native portable ZIP or Inno Setup installer and its checksum. |
| Test controls | Repository-provided synthetic/minimized fixture controls only. | Repository-provided synthetic/minimized fixture controls only. |
| Export location | New empty disposable directory that is outside every selected source path. | New empty disposable directory that is outside every selected source path. |

Do not use a victim disk image, private recovered material, credential, key, exploit sample, or executable payload. Do not point exports at the source directory. Do not run or open exported content as part of acceptance.

## Candidate-package preparation

### Linux x86_64

From a clean checked-out candidate on a native Linux x86_64 host, build the review bundle into a disposable output directory and verify it before launch:

```sh
sh scripts/package-linux-bundle.sh /absolute/path/to/review-output
sh scripts/verify-linux-bundle.sh \
  /absolute/path/to/review-output/DiskTrace-0.1.0-linux-x86_64.tar.gz
```

Record the archive filename, byte size, SHA-256 checksum, `release-manifest.json` source commit, exact launch path, and verifier result. Launch the extracted bundle through `launch-disktrace.sh`; do not treat a launcher smoke test as a completed acceptance result.

### Windows x86_64

Use a native Windows x86_64 host, not Wine, cross-target output, a browser/grid session, or a hosted CI log. Build and verify the portable ZIP with the commands in [Windows distribution v1](windows-distribution-v1.md). If testing the installer, compile it with the exact candidate version/source directory, verify the installer checksum, and install it into a new disposable per-user directory.

Record whether acceptance exercises the portable ZIP, the installer, or both. For an installer path, manually launch through the Start Menu and the installed `Start DiskTrace.cmd` launcher as applicable; the automated installer verifier does not establish these visible behaviors.

## Manual acceptance execution

Perform DA-01 through DA-10 in order using [Desktop acceptance v1](desktop-acceptance-v1.md). Record **pass**, **fail**, or **blocked** for every scenario. A pass is valid only for the exact recorded package, operating system, display conditions, controls, and commit.

| Checklist group | Reviewer action | Safe evidence to retain |
| --- | --- | --- |
| DA-01, DA-02, DA-09 | Launch the recorded package; use keyboard-only navigation; observe focus, labels, clipping, contrast, reduced window size, and visible recovery-scope boundaries. | Package/launch path, keys attempted, display/window conditions, and a redacted screenshot only when it contains no sensitive material. |
| DA-03, DA-04, DA-08 | Scan a supported synthetic source; observe cancellation/failure behavior and a safe source-identity mismatch control. | Fixture/control name, visible text, candidate count/refusal result, final state, and evidence that no source write occurred. |
| DA-05 | Review a selected candidate and a controlled preview failure where available. | Candidate/control identifier, preview state/error, and observation that recovered content was neither opened nor executed. |
| DA-06, DA-07 | Attempt an unsafe export destination, then export one selected result to the disposable destination; inspect receipt and case brief. | Refusal wording, confirmation text, disposable output path, receipt/case-brief metadata, and post-test cleanup observation. |
| DA-10 | Close and reopen through the documented session route, including invalid-session refusal where available. | Session control, visible result, identity-recheck behavior, and refusal text. |

Stop and record a **failure** if DiskTrace offers to write to the selected source, accepts its source directory as export destination, executes/renders recovered content, implies universal recovery, or exceeds the recorded platform scope. Stop and record **blocked** when a necessary test control, native package, native platform, or safe evidence path is unavailable.

## Record and review

Create one completed record per platform using the template in [Desktop acceptance v1](desktop-acceptance-v1.md). Attach only safe metadata and redacted evidence. The record must include the exact candidate SHA, package checksum, operating-system build, display/keyboard conditions, DA-01 through DA-10 outcomes, defects, and linked automated evidence.

A platform becomes eligible for first-release consideration only when its record contains no unresolved fail/block outcome for the advertised workflow. A manual record does not certify accessibility, all hardware, every operating-system version, complete recovery, or legal admissibility.

## Release boundary after acceptance

Manual results are one required input to a future release decision. After successful Linux and Windows records exist, the maintainer must still rebuild exact candidate artifacts, record checksums/SBOM/advisory disposition, refresh final notes and the controlled decision package, rerun all required hosted contexts on the intended target, and obtain new explicit owner authorization for the exact version, SHA, visibility, assets, and any signing/notarization action.

## Related documents

Use this kit with [Desktop acceptance v1](desktop-acceptance-v1.md), [Windows distribution v1](windows-distribution-v1.md), [Linux distribution v1](linux-distribution-v1.md), [Release process](release-process.md), [Release candidate record](release-candidate-v0.1.0.md), and [Safety and evidence boundaries](safety-and-evidence.md). The strictest applicable safety and authorization boundary governs.
