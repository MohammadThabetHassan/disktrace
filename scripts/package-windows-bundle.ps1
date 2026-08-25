[CmdletBinding()]
param(
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

if ([Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw 'Windows bundle creation must run on a Windows host.'
}
if (-not [Environment]::Is64BitOperatingSystem -or -not [Environment]::Is64BitProcess) {
    throw 'Windows bundle creation currently supports only x86_64 Windows hosts.'
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
Push-Location $Root
try {
    $dirtyState = git status --porcelain
    if ($LASTEXITCODE -ne 0 -or $dirtyState) {
        throw 'Windows bundle creation requires a clean committed source revision.'
    }
    $SourceCommit = (git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $SourceCommit -notmatch '^[0-9a-f]{40}$') {
        throw 'Unable to determine the committed source revision.'
    }
}
finally {
    Pop-Location
}
$Target = 'windows-x86_64'
$BundleName = "disktrace-$Version-$Target"
$ArchiveName = "DiskTrace-$Version-$Target.zip"
$ArchivePath = Join-Path $OutputDirectory $ArchiveName
$ChecksumPath = "$ArchivePath.sha256"
$StagingRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("disktrace-bundle-" + [guid]::NewGuid().ToString('N'))
$BundleDirectory = Join-Path $StagingRoot $BundleName

try {
    New-Item -ItemType Directory -Force -Path $OutputDirectory, (Join-Path $BundleDirectory 'bin'), (Join-Path $BundleDirectory 'docs') | Out-Null

    Push-Location $Root
    try {
        cargo build --release -p ef-cli -p ef-desktop
    }
    finally {
        Pop-Location
    }

    Copy-Item (Join-Path $Root 'target\release\evidenceforge.exe') (Join-Path $BundleDirectory 'bin\evidenceforge.exe')
    Copy-Item (Join-Path $Root 'target\release\evidenceforge-desktop.exe') (Join-Path $BundleDirectory 'bin\evidenceforge-desktop.exe')
    @(
        @{ Source = 'README.md'; Destination = 'docs\README.md' },
        @{ Source = 'LICENSE'; Destination = 'docs\LICENSE' },
        @{ Source = 'docs\safety-and-evidence.md'; Destination = 'docs\safety-and-evidence.md' },
        @{ Source = 'docs\architecture.md'; Destination = 'docs\architecture.md' },
        @{ Source = 'docs\release-process.md'; Destination = 'docs\release-process.md' },
        @{ Source = 'docs\release-candidate-v0.1.0.md'; Destination = 'docs\release-candidate-v0.1.0.md' },
        @{ Source = 'docs\dependency-advisories.md'; Destination = 'docs\dependency-advisories.md' },
        @{ Source = 'docs\windows-distribution-v1.md'; Destination = 'docs\windows-distribution-v1.md' },
        @{ Source = 'docs\local-release-evidence-v1.md'; Destination = 'docs\local-release-evidence-v1.md' },
        @{ Source = 'docs\case-brief-v1.md'; Destination = 'docs\case-brief-v1.md' },
        @{ Source = 'docs\future-github-launch-v1.md'; Destination = 'docs\future-github-launch-v1.md' },
        @{ Source = 'docs\release-notes-v0.1.0-draft.md'; Destination = 'docs\release-notes-v0.1.0-draft.md' },
        @{ Source = 'docs\project-status.md'; Destination = 'docs\project-status.md' }
    ) | ForEach-Object {
        Copy-Item (Join-Path $Root $_.Source) (Join-Path $BundleDirectory $_.Destination)
    }

    @'
@echo off
setlocal
start "" "%~dp0bin\evidenceforge-desktop.exe" %*
'@ | Set-Content -Path (Join-Path $BundleDirectory 'launch-evidenceforge.cmd') -Encoding ascii -NoNewline
    Copy-Item (Join-Path $BundleDirectory 'launch-evidenceforge.cmd') (Join-Path $BundleDirectory 'Start DiskTrace.cmd')

    $manifest = [ordered]@{
        schema_version = 1
        product = 'DiskTrace'
        version = $Version
        target = $Target
        format = 'zip'
        license = 'Apache-2.0'
        source_commit = $SourceCommit
        source_state = 'clean-committed'
        supported_build_host = 'Windows x86_64'
        primary_launcher = 'Start DiskTrace.cmd'
        included_binaries = @('bin/evidenceforge-desktop.exe', 'bin/evidenceforge.exe')
        intentional_limits = @(
            'Not Authenticode signed',
            'Not an MSI, Microsoft Store package, or automatic update channel',
            'Not validated for Windows on ARM',
            'Not a public release artifact'
        )
    }
    $manifest | ConvertTo-Json -Depth 4 | Set-Content -Path (Join-Path $BundleDirectory 'release-manifest.json') -Encoding utf8 -NoNewline

    $stagedFiles = Get-ChildItem -Path (Join-Path $BundleDirectory 'bin'), (Join-Path $BundleDirectory 'docs') -File -Recurse
    $stagedFiles += Get-Item (Join-Path $BundleDirectory 'launch-evidenceforge.cmd'), (Join-Path $BundleDirectory 'Start DiskTrace.cmd'), (Join-Path $BundleDirectory 'release-manifest.json')
    $checksumLines = $stagedFiles |
        Sort-Object { $_.FullName.Substring($BundleDirectory.Length + 1).Replace('\', '/') } |
        ForEach-Object {
            $relative = $_.FullName.Substring($BundleDirectory.Length + 1).Replace('\', '/')
            $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant()
            "$hash  $relative"
        }
    $checksumLines | Set-Content -Path (Join-Path $BundleDirectory 'SHA256SUMS') -Encoding ascii

    Remove-Item -Force -ErrorAction SilentlyContinue $ArchivePath, $ChecksumPath
    Compress-Archive -Path $BundleDirectory -DestinationPath $ArchivePath -CompressionLevel Optimal
    $archiveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $ArchivePath).Hash.ToLowerInvariant()
    "$archiveHash  $ArchiveName" | Set-Content -Path $ChecksumPath -Encoding ascii

    Write-Output "created $ArchivePath"
    Write-Output "created $ChecksumPath"
}
finally {
    Remove-Item -Force -Recurse -ErrorAction SilentlyContinue $StagingRoot
}
