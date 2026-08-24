# DiskTrace GitHub publication and release checklist, version 1

## Current publication scope

The owner has authorized an initial **public source repository** for DiskTrace and authorized the initial Git history to use the following commit identity:

> **MohammadThabetHassan <20220002188@students.cud.ac.ae>**

This authorization covers repository initialization, a public GitHub repository, the initial authored commit, and the initial push to the primary branch. It does **not** authorize a tag, GitHub Release, release-asset upload, production-support commitment, signing purchase, hosted deployment, or any statement that DiskTrace is production-ready.

## Materials included in the source publication

| Material | Public purpose |
| --- | --- |
| Apache-2.0 license, contribution guide, code of conduct, and security policy | Baseline open-source governance and responsible reporting route. |
| CI workflow definitions | Linux verification and native Windows build/installer checks to run on the hosted revision. |
| Source code, deterministic fixtures, and verification scripts | Inspectable implementation and reproducible local quality controls. |
| Safety, workflow, architecture, source-access, distribution, and dependency-advisory documents | Versioned scope, user, maintainer, and engineering boundaries. |
| `CHANGELOG.md`, draft release notes, and `docs/project-status.md` | Change history, future-release material, and the evidence-led current status. |

Locally generated artifacts, local advisory logs, screenshots, temporary files, and locally rebuilt distribution archives are deliberately excluded from the source repository. They are not public release assets.

## Required checks after initial publication

1. Inspect the GitHub repository and its primary branch after the initial push.
2. Wait for the Linux verification and native Windows distribution workflows on the pushed revision; treat a queued, failed, or unavailable workflow as incomplete evidence.
3. Configure proportional branch protection or rulesets only after choosing a direct-main or pull-request contribution policy. Require only checks that the repository actually produces.
4. Validate artifacts on their native supported hosts. This includes Linux bundle behavior and the Windows portable ZIP, launcher, installer, uninstaller, signing/trust state, and accessibility checks.
5. Confirm a security-reporting route, dependency-update posture, and maintainer ownership before inviting broad contributions.

## Release gates

A **public source repository** is not a public production release. A release remains blocked until the intended revision has hosted validation evidence, native platform validation, authorized signing decisions, release notes derived from verified changes, artifact checksums, a support statement, and owner approval. The Linux-host cross-target Windows ZIP must never be described as native Windows release evidence.

Before any future release, create a semantic tag and a draft release only after all relevant hosted checks pass. Attach only validated artifacts and their checksums, review the draft, then publish only with the owner’s separate explicit approval.
