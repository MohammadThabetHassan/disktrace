[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ArchivePath,
    [switch]$SkipDesktopSmoke
)

$ErrorActionPreference = 'Stop'

if ([Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw 'Windows bundle verification must run on a Windows host.'
}

$ArchivePath = [System.IO.Path]::GetFullPath($ArchivePath)
if (-not (Test-Path -LiteralPath $ArchivePath -PathType Leaf)) {
    throw "Bundle archive does not exist: $ArchivePath"
}
$ChecksumPath = "$ArchivePath.sha256"
if (-not (Test-Path -LiteralPath $ChecksumPath -PathType Leaf)) {
    throw "Bundle checksum does not exist: $ChecksumPath"
}

$expectedArchiveLine = (Get-Content -LiteralPath $ChecksumPath -Raw).Trim()
if ($expectedArchiveLine -notmatch '^([a-fA-F0-9]{64})  (.+)$') {
    throw "Bundle checksum is malformed: $ChecksumPath"
}
$actualArchiveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $ArchivePath).Hash.ToLowerInvariant()
if ($actualArchiveHash -ne $Matches[1].ToLowerInvariant()) {
    throw "Bundle checksum mismatch: $ArchivePath"
}

$Work = Join-Path ([System.IO.Path]::GetTempPath()) ("disktrace-verify-" + [guid]::NewGuid().ToString('N'))
try {
    Expand-Archive -LiteralPath $ArchivePath -DestinationPath $Work
    $bundle = Get-ChildItem -LiteralPath $Work -Directory | Where-Object { $_.Name -match '^disktrace-.+-windows-x86_64$' } | Select-Object -First 1
    if (-not $bundle) {
        throw "Archive does not contain the expected Windows bundle root: $ArchivePath"
    }

    $requiredPaths = @(
        'bin\evidenceforge.exe',
        'bin\evidenceforge-desktop.exe',
        'docs\README.md',
        'docs\LICENSE',
        'docs\safety-and-evidence.md',
        'docs\architecture.md',
        'docs\release-process.md',
        'docs\dependency-advisories.md',
        'docs\\windows-distribution-v1.md',
        'docs\\local-release-evidence-v1.md',
        'docs\\case-brief-v1.md',
        'docs\\future-github-launch-v1.md',
        'docs\\release-notes-v0.1.0-draft.md',
        'docs\\project-status.md',
        'launch-evidenceforge.cmd',
        'Start DiskTrace.cmd',
        'release-manifest.json',
        'SHA256SUMS'
    )
    foreach ($relativePath in $requiredPaths) {
        $path = Join-Path $bundle.FullName $relativePath
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Archive is missing required path: $relativePath"
        }
    }

    $primaryLauncherPath = Join-Path $bundle.FullName 'Start DiskTrace.cmd'
    if ((Get-Content -LiteralPath $primaryLauncherPath -Raw) -notmatch 'evidenceforge-desktop\\.exe') {
        throw 'Primary Windows launcher does not start the desktop application.'
    }

    $manifest = Get-Content -LiteralPath (Join-Path $bundle.FullName 'release-manifest.json') -Raw | ConvertFrom-Json
    if ($manifest.product -ne 'DiskTrace' -or
        $manifest.target -ne 'windows-x86_64' -or
        $manifest.license -ne 'Apache-2.0' -or
        $manifest.source_state -ne 'built-locally' -or
        $manifest.primary_launcher -ne 'Start DiskTrace.cmd') {
        throw 'Release manifest does not match the Windows distribution contract.'
    }

    foreach ($line in Get-Content -LiteralPath (Join-Path $bundle.FullName 'SHA256SUMS')) {
        if ($line -notmatch '^([a-fA-F0-9]{64})  (.+)$') {
            throw "Malformed staged-file checksum: $line"
        }
        $expectedHash = $Matches[1].ToLowerInvariant()
        $relativePath = $Matches[2]
        $path = Join-Path $bundle.FullName ($relativePath -replace '/', '\\')
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Checksum references a missing staged file: $relativePath"
        }
        $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        if ($actualHash -ne $expectedHash) {
            throw "Staged-file checksum mismatch: $relativePath"
        }
    }

    & (Join-Path $bundle.FullName 'bin\evidenceforge.exe') --help | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Packaged CLI returned exit code $LASTEXITCODE for --help."
    }

    if ($SkipDesktopSmoke) {
        Write-Output 'bundle desktop smoke launch skipped by request'
    }
    else {
        $process = Start-Process -FilePath (Join-Path $bundle.FullName 'bin\\evidenceforge-desktop.exe') -PassThru
        Start-Sleep -Seconds 3
        if ($process.HasExited) {
            throw "Packaged desktop exited during the Windows smoke check with exit code $($process.ExitCode)."
        }
        Stop-Process -Id $process.Id -Force
        Write-Output 'bundle desktop smoke launch passed'
    }

    Write-Output 'Windows distribution bundle verification passed'
}
finally {
    Remove-Item -Force -Recurse -ErrorAction SilentlyContinue $Work
}
