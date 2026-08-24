# DiskTrace: cross-platform validation record

**Assessment date:** 2026-08-18  
**Scope:** Local Linux x86_64 execution, Windows GNU cross-compilation, and Windows emulation. No native Windows machine, Windows installer run, hosted Windows runner, repository, commit, or publication was used.

## Verdict

DiskTrace is **confirmed working end to end on the tested Linux x86_64 environment** for its currently documented recovery scope and deterministic fixtures. The packaged GUI was launched, interacted with through the guided workflow, scanned a synthetic image, displayed validated results, selected a candidate, exported it to a separate folder, created a receipt, and produced bytes that matched the expected fixture exactly. The packaged CLI also completed nine byte-for-byte recovery scenarios spanning FAT12, FAT16, PNG, JPEG, PDF, ZIP/Open XML, exFAT, NTFS resident, and NTFS contiguous recovery.

The Windows implementation has strong preliminary evidence but is **not yet certified as 100% working on native Windows**. The full workspace type-checks for Windows, release `.exe` artifacts are built and structurally valid, the CLI completed a fixture recovery under a Windows compatibility runtime, and the desktop application rendered and scanned successfully under that runtime. Native Windows packaging, the Inno Setup installer, an MSVC build, and a real Windows desktop session have not been executed in this Linux-only environment.

> “100% working” can only mean 100% of a defined and executed test scope. It cannot honestly mean every possible drive, recovery condition, Windows configuration, or user environment.

| Platform | Honest status | What is confirmed | What remains |
|---|---|---|---|
| Linux x86_64 | **End-to-end validated for the local test scope** | Packaged CLI, packaged GUI, recovery bytes, source immutability, safe destination refusal, overwrite refusal, receipts, checksums, and GUI workflow. | Native manual accessibility review, real customer images, additional Linux distributions, and signed-release validation. |
| Windows x86_64 | **Cross-built and emulation-validated; native validation pending** | Windows-target type check, release PE executables, imported Windows libraries, Wine CLI fixture recovery, Wine desktop rendering, and Wine guided-demo scan. | Native MSVC build, PowerShell package script, Inno Setup installer, installer install/uninstall, real Windows GUI and file dialog, Defender/SmartScreen behavior, and signing. |

## Executed Linux evidence

The rebuilt archive `DiskTrace-0.1.0-linux-x86_64.tar.gz` passed archive and staged-file checksum verification, CLI help validation, required-document validation, and a desktop-launch smoke check. The archive SHA-256 is:

```text
21fadca5b67d1deacde3c16787583837b3f72086df6f2f52776228b5954482b8
```

The packaged CLI was then used directly, rather than the workspace binary, to scan all deterministic source images. It recovered and compared the following nine outputs byte for byte against their expected artifacts.

| Recovery coverage | Verified outputs |
|---|---:|
| Deleted FAT12 metadata and PNG carving | 2 |
| Deleted FAT16 metadata and JPEG carving | 2 |
| PDF and ZIP/Open XML carving | 2 |
| Deleted exFAT contiguous root metadata | 1 |
| Deleted NTFS resident record | 1 |
| Deleted NTFS contiguous non-resident extent | 1 |
| **Total** | **9** |

The same packaged CLI correctly rejected a destination on the source image’s storage location and refused to overwrite an existing recovered file. SHA-256 values of all six tested source images were unchanged before and after recovery operations.

The packaged GUI was exercised under Xvfb using X11 input automation. The visual record confirms the initial safe-recovery interface, a completed guided-demo scan with a source-verified evidence session and two candidates, candidate selection with method explanation and bounded preview, and a completed export. The final state visibly reported the recovered file path, receipt path, and one recorded export. The exported FAT12 result matched the deterministic expected file exactly and its receipt JSON was present.

## Executed Windows evidence

The project passed:

```text
cargo check --workspace --all-targets --target x86_64-pc-windows-gnu
cargo build --release -p ef-cli -p ef-desktop --target x86_64-pc-windows-gnu
sh scripts/verify-windows-config.sh
```

The cross-build produced 64-bit Windows PE executables for both the CLI and desktop application. PE inspection showed the desktop program imports the expected Windows system, user-interface, OpenGL, shell, and accessibility libraries, while the CLI imports its expected core system libraries.

Under Wine, the Windows CLI successfully displayed help, scanned the FAT12 fixture, found both expected candidates, recovered `fat12-root-0000`, emitted a receipt, and produced bytes identical to the expected recovered file. The Windows desktop executable rendered its DiskTrace interface under Xvfb. A guided-demo interaction reached the visible read-only scan-complete state, source-verified session, and two expected recovery candidates.

## Required native Windows completion test

Run the following on an actual Windows x86_64 host with Rust 1.97.1 and Inno Setup 6 installed. The commands build the distribution, validate archive checksums and desktop launch, and create the per-user installer.

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\package-windows-bundle.ps1
.\scripts\verify-windows-bundle.ps1 -ArchivePath .\dist\DiskTrace-0.1.0-windows-x86_64.zip
.\scripts\build-windows-installer.ps1
```

Then perform manual acceptance in a normal Windows desktop session. Confirm that the portable launcher opens the GUI, the native file picker selects a synthetic fixture, the guided scan and export complete, the receipt appears, the installer creates its Start Menu and optional desktop shortcuts, the application uninstalls cleanly, and the package behavior is acceptable to Windows Defender and SmartScreen. Record the Windows version, hardware architecture, Rust version, installer hash, and test result with the release candidate.

## Conclusion

Linux is genuinely working for the tested product scope, including a complete GUI recovery path and every implemented recovery method’s deterministic fixture. Windows is in a strong pre-native-validation state, not a finished Windows certification. The correct next step is one native Windows x86_64 validation run; after it passes, the project can accurately claim tested Windows and Linux distribution support for its documented scope.
