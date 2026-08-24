# Windows distribution contract, version 1

DiskTrace defines a **native Windows x86_64** distribution path made of two artifacts: a portable ZIP bundle and an optional Inno Setup installer. Both are built and verified on a Windows host. This workspace provides scripts and a future hosted Windows workflow, but the current Linux environment does not represent Windows build, launch, installer, or SmartScreen evidence.

> A Windows configuration file is not evidence of a Windows release. The ZIP and installer become release candidates only after the native Windows workflow completes on the intended source revision and its artifacts are checked on Windows.

## Artifact boundary

| Artifact | Native build path | Contents | Verification boundary |
|---|---|---|---|
| **Portable ZIP** | `scripts/package-windows-bundle.ps1` on Windows x86_64 | A clearly named root GUI launcher, desktop and CLI `.exe` files, public documents, manifest, staged-file checksums, archive checksum. | Archive shape, SHA-256 checksums, GUI-launcher wiring, optional CLI help, manifest, and optional desktop launch on a Windows host. |
| **Inno Setup installer** | `installer/windows/evidenceforge.iss` compiled with `ISCC.exe` on Windows. | Per-user install, Start Menu shortcut, uninstaller, desktop and CLI binaries, required documents. | Installer compilation, checksum, disposable per-user silent install/uninstall acceptance, installed-file/manifest/CLI-help checks, and uninstall-registration removal on a native Windows host. |
| **Hosted validation** | `.github/workflows/windows-release.yml` after an authorized repository is published. | Windows-native tests, ZIP build/verify, installer build, and artifact upload for review. | Hosted workflow result on the exact commit; uploaded artifacts remain review artifacts, not an automatically published release. |

The Inno Setup command-line compiler is `ISCC.exe`, and supports explicit output directory and filename options; the installer script uses this supported compiler boundary.[1] The Windows workflow uses a native GitHub-hosted runner and standard Rust build/test commands, consistent with GitHub’s Rust workflow guidance.[2]

## Supported platform and explicit refusals

| Dimension | Current contract |
|---|---|
| **Target** | Windows 10 or later, x86_64, built on a Windows x86_64 host. |
| **Toolchain** | Workspace-pinned Rust `1.97.1`. |
| **Install scope** | Per-user installation under the local application directory. Administrator elevation is not required or requested. |
| **Network behavior** | The installer and application do not need a cloud recovery account, upload path, or telemetry service. |
| **Signing** | No Authenticode signature is configured. Windows trust warnings are possible until an authorized signing identity and release process are established. |
| **Unsupported** | Windows on ARM, Windows versions before 10, Microsoft Store packaging, MSI, automatic updates, enterprise deployment policy, code signing, and notarized/reputation evidence. |

The portable ZIP opens through `Start DiskTrace.cmd`, which starts the native desktop workspace. The CLI remains inside `bin/` for optional scripted evidence workflows but is not the normal end-user path. The installer uses a product name without a version in the `AppName` field and sets the release version independently, following the Inno Setup directive model.[3] The package is explicitly pre-release and does not make claims about legal admissibility, completion of recovered bytes, malware safety, or universal filesystem support.

## Local Windows procedure

On a Windows x86_64 development host with the pinned Rust toolchain:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\package-windows-bundle.ps1
.\scripts\verify-windows-bundle.ps1 -ArchivePath .\dist\DiskTrace-0.1.0-windows-x86_64.zip
```

To compile the installer, install Inno Setup 6 on that Windows build host, then run:

```powershell
& "$env:ProgramFiles(x86)\Inno Setup 6\ISCC.exe" `
  "/DAppVersion=0.1.0" `
  "/DSourceDir=$PWD\dist\evidenceforge-0.1.0-windows-x86_64" `
  "/O$PWD\dist" `
  "/FDiskTrace-0.1.0-windows-x86_64-setup" `
  "$PWD\installer\windows\evidenceforge.iss"
```

The ZIP builder calculates a SHA-256 manifest for staged files and an archive checksum. The installer build calculates a separate installer checksum. Store those values with an eventual release record, but do not treat a checksum as a code signature.

## Native installer acceptance gate

`scripts/verify-windows-installer.ps1` is a native Windows-only acceptance gate invoked by the hosted Windows workflow after installer creation and before artifact upload. It verifies the installer checksum, installs once into a unique disposable per-user directory, confirms the installed binaries, selected public documents, manifest, checksum list, and uninstaller, invokes only the installed CLI’s `--help` surface, confirms one matching per-user uninstall registration, runs the uninstaller, and verifies both installation-directory and registration removal.

The gate uses Inno Setup’s documented `/VERYSILENT`, `/SUPPRESSMSGBOXES`, `/SP-`, `/NORESTART`, `/DIR`, and `/LOG` parameters for the installer, and its documented silent/uninstall logging and no-restart parameters for the uninstaller.[4] [5] The logs are presence checks only; Inno Setup documents them as diagnostic output rather than a stable machine-parsable format, so the verifier does not make semantics depend on log text.[4] [5]

> A passing installer acceptance gate proves one clean hosted runner completed the defined package mechanics. It does **not** launch the GUI, certify real-user usability or accessibility, validate SmartScreen or code signing, test upgrades, test arbitrary existing machines, or create a public release.

## Linux-host cross-target compatibility smoke

When a Linux development environment has the pinned `x86_64-pc-windows-gnu` Rust target, a compatible MinGW linker, Wine, and Xvfb, run:

```sh
sh scripts/verify-windows-cross-target.sh
```

The script cross-builds the release desktop and CLI binaries, requires the Windows CLI command surface under Wine, and requires the desktop process to remain running during a bounded Wine/X11 interval. This is useful local compatibility evidence for the binaries; it is **not** evidence of a Windows-native build, portable-ZIP launcher, installer, installation/uninstallation, signing, SmartScreen, accessibility, or usability outcome. Those claims remain reserved for native Windows validation.

## Linux-host portable cross-target ZIP

A Linux x86_64 environment that has the pinned `x86_64-pc-windows-gnu` target, MinGW linker, Wine, Xvfb, `zip`, and `unzip` can build a portable review artifact:

```sh
sh scripts/package-windows-cross-target.sh
sh scripts/verify-windows-cross-target-bundle.sh \
  dist/DiskTrace-0.1.0-windows-x86_64-cross-target.zip
```

The resulting ZIP contains the actual Windows-targeted desktop and CLI binaries, a conventional `.cmd` desktop launcher, user-facing documents, a manifest that names the Linux cross-target/Wine evidence boundary, staged-file SHA-256 checksums, and an archive checksum. The verifier checks the packaged command surface and bounded desktop process under Wine/Xvfb. It is useful for local compatibility review, but is **not** native Windows build, launcher, installer, signing, SmartScreen, accessibility, or clean-machine evidence.

After packaging both locally verified Linux and cross-target Windows artifacts, produce a machine-readable local evidence record with:

```sh
sh scripts/generate-local-release-evidence.sh \
  dist/DiskTrace-0.1.0-linux-x86_64.tar.gz \
  dist/DiskTrace-0.1.0-windows-x86_64-cross-target.zip
```

Read [the local release-evidence contract](local-release-evidence-v1.md) before relying on this record in a portfolio or release review.

## Hosted workflow boundary

The hosted Windows workflow builds and validates the project on `windows-2022`, packages the ZIP, compiles the installer with Inno Setup, verifies the ZIP and disposable installer install/uninstall path, calculates installer checksums, and uploads the artifacts for review. It has read-only repository permissions and no release-publishing step. Each result must be tied to its exact source revision; it is not a semantic-versioned or publicly distributed release.

## Publication prerequisites

Before a Windows artifact is distributed publicly, verify the workflow on the release commit, test portable ZIP and installer installation/uninstallation on a clean Windows machine, choose and configure an authorized signing process if appropriate, provide a real private security-reporting route, publish checksums and release notes, and state unsupported Windows configurations plainly. See [Release process](release-process.md) and [Safety and evidence boundaries](safety-and-evidence.md).

## References

[1] [Inno Setup command-line compiler](https://jrsoftware.org/ishelp/topic_compilercmdline.htm)

[2] [GitHub Docs: Building and testing Rust](https://docs.github.com/actions/tutorials/build-and-test-code/building-and-testing-rust)

[3] [Inno Setup `AppName` directive](https://jrsoftware.org/ishelp/topic_setup_appname.htm)

[4] [Inno Setup setup command-line parameters](https://jrsoftware.org/ishelp/topic_setupcmdline.htm)

[5] [Inno Setup uninstaller command-line parameters](https://jrsoftware.org/ishelp/topic_uninstcmdline.htm)
