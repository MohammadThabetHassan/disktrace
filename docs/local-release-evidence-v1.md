# Local release-evidence contract, version 1

DiskTrace can generate a **local evidence record** for a pair of verified Linux and Windows cross-target portable artifacts. The record is designed for internal review and portfolio demonstration. It does not create a tag, signature, GitHub release, hosted workflow result, public URL, or native Windows release assertion.

## What the evidence generator verifies

Before it writes a record, the generator runs the existing Linux bundle verifier and the Windows cross-target portable bundle verifier. A successful record therefore contains the SHA-256 values and byte sizes of artifacts whose archive structure, staged-file checksums, and bounded executable smoke checks have passed in the local environment.

| Artifact | Required verification | Evidence boundary |
| --- | --- | --- |
| Linux x86_64 `tar.gz` | Archive checksum, staged checksums, CLI help, native Linux desktop smoke. | Local Linux x86_64 build and launch evidence. |
| Windows x86_64 cross-target ZIP | Archive checksum, staged checksums, launcher wiring, packaged CLI command surface under Wine, and bounded packaged desktop process under Wine/Xvfb. | Linux-host cross-target and compatibility evidence only. |

## Generation procedure

After packaging the Linux bundle and the Windows cross-target portable ZIP, run:

```sh
sh scripts/generate-local-release-evidence.sh \
  dist/DiskTrace-0.1.0-linux-x86_64.tar.gz \
  dist/DiskTrace-0.1.0-windows-x86_64-cross-target.zip
```

The command writes `dist/DiskTrace-0.1.0-local-evidence.json`. The JSON record includes the local UTC generation time, host operating-system and architecture labels, artifact hashes and byte sizes, commands that were executed, and intentional limitations.

## Explicit non-claims

The record must not be described as a public release record or a code-signing attestation. It cannot replace a native Windows portable-ZIP and installer check, Windows signing and SmartScreen evidence, macOS validation, hosted CI on the release revision, branch protection, a private security-reporting route, a semantic tag, release notes, a public checksum page, or a published release.

> Local evidence establishes what was verified in this environment. It never establishes properties that were not tested here.
