#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

for path in \
    docs/windows-distribution-v1.md \
    scripts/package-windows-bundle.ps1 \
    scripts/verify-windows-bundle.ps1 \
    scripts/build-windows-installer.ps1 \
    scripts/verify-windows-installer.ps1 \
    installer/windows/evidenceforge.iss \
    .github/workflows/windows-release.yml; do
    test -s "$path"
done

grep -q 'Windows bundle creation must run on a Windows host' scripts/package-windows-bundle.ps1
grep -q 'evidenceforge-desktop.exe' scripts/package-windows-bundle.ps1
grep -q 'Start DiskTrace.cmd' scripts/package-windows-bundle.ps1
grep -q 'primary_launcher' scripts/package-windows-bundle.ps1
grep -q 'release-manifest.json' scripts/package-windows-bundle.ps1
grep -q 'dependency-advisories.md' scripts/package-windows-bundle.ps1
grep -q 'Get-FileHash -Algorithm SHA256' scripts/package-windows-bundle.ps1
grep -q 'Windows bundle verification must run on a Windows host' scripts/verify-windows-bundle.ps1
grep -q 'SkipDesktopSmoke' scripts/verify-windows-bundle.ps1
grep -q 'Start DiskTrace.cmd' scripts/verify-windows-bundle.ps1
grep -q 'dependency-advisories.md' scripts/verify-windows-bundle.ps1
grep -q 'Inno Setup 6 was not found' scripts/build-windows-installer.ps1
grep -q 'ISCC.exe' scripts/build-windows-installer.ps1
grep -q 'Windows installer acceptance verification must run on a Windows host' scripts/verify-windows-installer.ps1
grep -q 'Installer checksum mismatch' scripts/verify-windows-installer.ps1
grep -q 'unins000.exe' scripts/verify-windows-installer.ps1
grep -q 'Get-InstallEntries' scripts/verify-windows-installer.ps1
grep -q '/VERYSILENT' scripts/verify-windows-installer.ps1
grep -q 'Installed CLI returned exit code' scripts/verify-windows-installer.ps1
grep -q 'PrivilegesRequired=lowest' installer/windows/evidenceforge.iss
grep -q 'ArchitecturesAllowed=x64compatible' installer/windows/evidenceforge.iss
grep -q 'UninstallDisplayName=DiskTrace' installer/windows/evidenceforge.iss
if grep -q 'example.invalid' installer/windows/evidenceforge.iss; then
    printf '%s\n' 'installer configuration contains a placeholder public URL' >&2
    exit 1
fi

grep -q 'runs-on: windows-2022' .github/workflows/windows-release.yml
grep -q 'contents: read' .github/workflows/windows-release.yml
grep -q 'cargo test --workspace' .github/workflows/windows-release.yml
grep -q 'cargo clippy --workspace --all-targets -- -D warnings' .github/workflows/windows-release.yml
grep -q 'package-windows-bundle.ps1' .github/workflows/windows-release.yml
grep -q 'verify-windows-bundle.ps1' .github/workflows/windows-release.yml
grep -q 'choco install innosetup' .github/workflows/windows-release.yml
grep -q 'build-windows-installer.ps1' .github/workflows/windows-release.yml
grep -q 'verify-windows-installer.ps1' .github/workflows/windows-release.yml
grep -q 'Verify installer install and uninstall' .github/workflows/windows-release.yml
grep -q 'actions/upload-artifact@v4' .github/workflows/windows-release.yml

printf '%s\n' 'Windows distribution configuration verification passed'
