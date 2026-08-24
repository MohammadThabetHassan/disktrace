# Linux distribution bundle contract, version 1

DiskTrace can produce a **local Linux x86_64 desktop bundle** from this workspace. The bundle is a versioned `tar.gz` archive containing the native desktop application, command-line application, required user-facing documents, a machine-readable local-build manifest, and SHA-256 checksums. It is a distribution artifact, not a signed installer or public release.

## Supported build boundary

| Dimension | Supported by this contract |
|---|---|
| **Build host** | Linux x86_64 only. The packaging script refuses another operating system or architecture rather than emitting a misleading artifact. |
| **Rust toolchain** | Workspace-pinned Rust `1.97.1` with the release profile. |
| **Applications** | `evidenceforge-desktop` native desktop workspace and `evidenceforge` command-line interface. |
| **Archive type** | `tar.gz`, staged under `evidenceforge-<version>-linux-x86_64/`. |
| **Destination** | A caller-selected output directory, defaulting to `dist/` in the workspace. |
| **Integrity data** | One SHA-256 manifest for staged files and one checksum for the final archive. |

The contract does **not** claim support for Windows, macOS, ARM Linux, universal packages, automatic updates, code signing, notarization, sandbox permissions, distribution repositories, or installer integration. Those require platform-native artifact builds and tests on their own supported hosts.

## Bundle layout

```text
DiskTrace-<version>-linux-x86_64.tar.gz
└── evidenceforge-<version>-linux-x86_64/
    ├── bin/
    │   ├── evidenceforge-desktop
    │   └── evidenceforge
    ├── launch-evidenceforge.sh
    ├── install-desktop-launcher.sh
    ├── docs/
    │   ├── README.md
    │   ├── LICENSE
    │   ├── safety-and-evidence.md
    │   ├── architecture.md
    │   └── release-process.md
    ├── release-manifest.json
    └── SHA256SUMS
```

The archive root includes `launch-evidenceforge.sh`, the **primary GUI launcher**, and `install-desktop-launcher.sh`, which writes a per-user desktop entry under the XDG applications directory. The command-line binary remains available in `bin/` for optional automation and evidence workflows, but it is not the normal end-user launch path.

The release manifest records the workspace version, target label, license identifier, packaging format, source state (`local-uncommitted` for a local build), primary launcher, and support boundary. It does not claim a Git revision, signature, publication date, or published release status that the local workspace cannot prove.

## Build procedure

Run:

```sh
sh scripts/package-linux-bundle.sh
```

The script runs the release build for the desktop and CLI packages, stages only the listed distribution files, writes a static local-build manifest, calculates staged-file checksums, creates a deterministically ordered archive when GNU tar is available, and writes a checksum for the final archive.

## Verification procedure

Run:

```sh
sh scripts/verify-linux-bundle.sh dist/DiskTrace-0.1.0-linux-x86_64.tar.gz
```

The verifier checks the archive shape, verifies every staged-file SHA-256 checksum, inspects the manifest contract, confirms the GUI launcher and both binaries are executable, confirms the optional CLI starts with `--help`, and performs the desktop smoke launch through the GUI launcher with `xvfb-run` when available. A missing `xvfb-run` is reported as an explicit local skip; the checksum and CLI checks still run.

## Security and operator boundaries

The bundle contains no disk images, recovered bytes, saved sessions, telemetry configuration, credentials, or code-signing keys. The binaries maintain the same local-only source handling, destination policy, source-integrity checks, and recovery limitations documented in the repository. Read [Safety and evidence boundaries](safety-and-evidence.md) before handling a recovered export.

## Release path after local packaging

A public release requires platform-native build evidence, artifact checksums, hosted CI on the release commit, signed or otherwise authenticated distribution decisions, repository governance, a private security-reporting route, and release notes. See [Release process](release-process.md). A local `tar.gz` created by this contract is preparatory evidence only.
