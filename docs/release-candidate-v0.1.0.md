# DiskTrace v0.1.0 release-candidate record

> **Status: preparation only.** This record does not create or authorize a semantic tag, GitHub Release, public artifact, code-signing action, notarization action, support commitment, or production-release claim.

## Candidate intent

The proposed first public version is **v0.1.0**, scoped as a local-first forensic recovery workspace. The final release target has **not** yet been selected; it must be a clean commit on protected `main` after all candidate metadata, versioned artifacts, manual acceptance, local checks, and exact-SHA hosted evidence are complete.

| Field | Current record |
| --- | --- |
| Proposed version | `v0.1.0` |
| Release state | Preparation only; no tag and no GitHub Release exist. |
| Candidate source target | Not yet frozen. The final target must be recorded after all release-candidate changes are committed and verified. |
| Authorized source identity | `MohammadThabetHassan <20220002188@students.cud.ac.ae>` for all project commits. |
| Release visibility | Not yet authorized. |
| Signing and notarization | Not authorized; no signing or notarization action is planned. |
| Dependabot pull requests | #10 and #11 remain unmerged and are not part of this candidate unless separately approved and fully reverified. |

## Proposed public scope

| Area | Allowed statement if the final evidence gate passes | Prohibited statement |
| --- | --- | --- |
| Product | DiskTrace is a local-only desktop workflow for inspecting disk-image files, reviewing bounded supported candidates, and exporting a selected candidate to a separately approved destination. | It is not a universal recovery, device-acquisition, data-authenticity, malware-safety, legal-admissibility, or professional-forensics guarantee. |
| Recovery methods | Deleted FAT12/FAT16 root metadata, bounded contiguous exFAT root metadata, narrow NTFS resident/contiguous cases, and structural PNG/JPEG/GIF/AVI/limited MP4-MOV/PDF/ZIP-OpenXML carving under their method contracts. | No fragmented, overwritten, encrypted, TRIM/controller-discarded, generic formatted-drive, or complete-recovery claim. |
| Windows x86_64 | A versioned artifact only if exact-target native Windows artifact and manual-acceptance evidence are completed and recorded. | No SmartScreen, Authenticode, upgrade, accessibility, or broad hardware claim without specific evidence. |
| Linux x86_64 | A versioned artifact only if exact-target Linux artifact and manual-acceptance evidence are completed and recorded. | No signed-package, universal distribution, or general desktop compatibility claim. |
| macOS | Excluded from consumer artifact scope by default. Existing macOS 14 ARM64 CI is review-binary validation evidence only. | No consumer `.app`, installer, Intel support, signing, notarization, Gatekeeper, or manual-acceptance claim. |

## Required decision evidence

| Gate | Candidate requirement | Current state |
| --- | --- | --- |
| Source hygiene | Clean target tree, protected `main`, exact authorized identity, and no sensitive/generated source content. | Pending final target selection. |
| Local verification | `sh scripts/verify-all.sh`, artifact verifiers, checksum verification, release-document contract, and dependency review pass on the exact target. | Pending final target selection. |
| Hosted verification | Linux, Windows, macOS 14 ARM64, and CodeQL required contexts pass on the exact target. | Pending final target selection. |
| Artifact manifest | Versioned artifact names, byte sizes, SHA-256 checksums, SBOM/review metadata, and declared support scope are recorded. | Not started. |
| Manual acceptance | DA-01 through DA-10 are observed and safely recorded for every advertised artifact/platform. | Not started; automated companion checks are not manual acceptance. |
| Release notes | Final notes list changes, scope, safety controls, known limitations, advisory disposition, artifact hashes, and signing state. | Draft maintained; final review pending. |
| Publication authorization | Owner identifies exact SHA, semantic version, visibility, approved assets, and signing/notarization permissions. | Not granted. |

## Candidate verification commands

Run from the workspace root after the final candidate commit is selected:

```sh
sh scripts/verify-all.sh
sh scripts/verify-release-docs.sh
sh scripts/verify-linux-bundle.sh <versioned-linux-artifact>
sh scripts/verify-windows-cross-target-bundle.sh <versioned-windows-artifact>
sh scripts/generate-local-release-evidence.sh <versioned-linux-artifact> <versioned-windows-artifact>
sh scripts/generate-sbom.sh <dedicated-sbom-output-directory>
```

The Linux-host Windows cross-target verifier is compatibility evidence only. It does not replace the exact native Windows hosted package evidence or an actual manual Windows acceptance record.

## Approval boundary

The release decision may be requested only after every required record above is complete for one exact source target. A final authorization must expressly approve the exact version, commit SHA, tag creation, public or draft release state, attachment of named assets/checksums, and any signing/notarization action. No action is implied by this preparation record.

## Related documents

Use this record with the [release process](release-process.md), [release scorecard](release-scorecard-v1.md), [controlled release decision](release-decision-v1.md), [draft v0.1.0 release notes](release-notes-v0.1.0-draft.md), [desktop acceptance checklist](desktop-acceptance-v1.md), [local release-evidence contract](local-release-evidence-v1.md), and [project status](project-status.md). The strictest applicable safety and authorization boundary governs.
