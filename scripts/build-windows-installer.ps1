[CmdletBinding()]
param(
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

if ([Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw 'Windows installer creation must run on a Windows host.'
}
if (-not [Environment]::Is64BitOperatingSystem -or -not [Environment]::Is64BitProcess) {
    throw 'Windows installer creation currently supports only x86_64 Windows hosts.'
}

if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $Root 'dist'
}
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)

$cargoToml = Get-Content (Join-Path $Root 'Cargo.toml') -Raw
$versionMatch = [regex]::Match($cargoToml, '(?m)^version = "([^"]+)"')
if (-not $versionMatch.Success) {
    throw 'Unable to determine workspace version from Cargo.toml.'
}
$Version = $versionMatch.Groups[1].Value
$ArchivePath = Join-Path $OutputDirectory "DiskTrace-$Version-windows-x86_64.zip"
$InstallerPath = Join-Path $OutputDirectory "DiskTrace-$Version-windows-x86_64-setup.exe"

$compilerCandidates = @(
    (Get-Command ISCC.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -ErrorAction SilentlyContinue),
    (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
    (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe')
) | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Leaf) }
$Compiler = $compilerCandidates | Select-Object -First 1
if (-not $Compiler) {
    throw 'Inno Setup 6 was not found. Install it, add ISCC.exe to PATH, then rerun this script.'
}

& (Join-Path $Root 'scripts\package-windows-bundle.ps1') -OutputDirectory $OutputDirectory
& (Join-Path $Root 'scripts\verify-windows-bundle.ps1') -ArchivePath $ArchivePath -SkipDesktopSmoke

$Work = Join-Path ([System.IO.Path]::GetTempPath()) ("disktrace-installer-" + [guid]::NewGuid().ToString('N'))
try {
    Expand-Archive -LiteralPath $ArchivePath -DestinationPath $Work
    $bundle = Get-ChildItem -LiteralPath $Work -Directory | Where-Object { $_.Name -eq "disktrace-$Version-windows-x86_64" } | Select-Object -First 1
    if (-not $bundle) {
        throw "Portable archive did not contain the expected bundle directory: $ArchivePath"
    }

    Remove-Item -Force -ErrorAction SilentlyContinue $InstallerPath, "$InstallerPath.sha256"
    & $Compiler "/DAppVersion=$Version" "/DSourceDir=$($bundle.FullName)" "/O$OutputDirectory" "/FEDiskTrace-$Version-windows-x86_64-setup" (Join-Path $Root 'installer\windows\evidenceforge.iss')
    if ($LASTEXITCODE -ne 0) {
        throw "Inno Setup compilation returned exit code $LASTEXITCODE."
    }
    if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) {
        throw "Inno Setup did not produce the expected installer: $InstallerPath"
    }

    $installerHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $InstallerPath).Hash.ToLowerInvariant()
    "$installerHash  $(Split-Path -Leaf $InstallerPath)" | Set-Content -Path "$InstallerPath.sha256" -Encoding ascii
    Write-Output "created $InstallerPath"
    Write-Output "created $InstallerPath.sha256"
}
finally {
    Remove-Item -Force -Recurse -ErrorAction SilentlyContinue $Work
}
