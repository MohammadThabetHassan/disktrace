[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$InstallerPath
)

$ErrorActionPreference = 'Stop'

if ([Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT) {
    throw 'Windows installer acceptance verification must run on a Windows host.'
}
if (-not [Environment]::Is64BitOperatingSystem -or -not [Environment]::Is64BitProcess) {
    throw 'Windows installer acceptance verification currently supports only x86_64 Windows hosts.'
}

$InstallerPath = [System.IO.Path]::GetFullPath($InstallerPath)
if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) {
    throw "Installer does not exist: $InstallerPath"
}

$ChecksumPath = "$InstallerPath.sha256"
if (-not (Test-Path -LiteralPath $ChecksumPath -PathType Leaf)) {
    throw "Installer checksum does not exist: $ChecksumPath"
}
$checksumLine = (Get-Content -LiteralPath $ChecksumPath -Raw).Trim()
if ($checksumLine -notmatch '^([a-fA-F0-9]{64})  (.+)$') {
    throw "Installer checksum is malformed: $ChecksumPath"
}
$actualInstallerHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $InstallerPath).Hash.ToLowerInvariant()
if ($actualInstallerHash -ne $Matches[1].ToLowerInvariant()) {
    throw "Installer checksum mismatch: $InstallerPath"
}

function Get-InstallEntries([string]$ExpectedInstallRoot) {
    $expected = [System.IO.Path]::GetFullPath($ExpectedInstallRoot).TrimEnd('\')
    @(
        Get-ChildItem -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall' -ErrorAction SilentlyContinue |
            ForEach-Object {
                $entry = Get-ItemProperty -LiteralPath $_.PSPath
                if ($entry.InstallLocation -and $entry.InstallLocation.TrimEnd('\') -ieq $expected) {
                    $entry
                }
            }
    )
}

$Work = Join-Path ([System.IO.Path]::GetTempPath()) ("disktrace-installer-acceptance-" + [guid]::NewGuid().ToString('N'))
$InstallRoot = Join-Path $Work 'installed'
$InstallLog = Join-Path $Work 'install.log'
$UninstallLog = Join-Path $Work 'uninstall.log'
$UninstallerPath = $null

try {
    New-Item -ItemType Directory -Path $Work | Out-Null

    $installArguments = "/VERYSILENT /SUPPRESSMSGBOXES /SP- /NORESTART /DIR=`"$InstallRoot`" /LOG=`"$InstallLog`""
    $installProcess = Start-Process -FilePath $InstallerPath -ArgumentList $installArguments -Wait -PassThru
    if ($installProcess.ExitCode -ne 0) {
        throw "Installer returned exit code $($installProcess.ExitCode)."
    }
    if (-not (Test-Path -LiteralPath $InstallLog -PathType Leaf)) {
        throw 'Installer did not create the requested acceptance log.'
    }

    $requiredPaths = @(
        'bin\evidenceforge.exe',
        'bin\evidenceforge-desktop.exe',
        'docs\README.md',
        'docs\safety-and-evidence.md',
        'launch-evidenceforge.cmd',
        'release-manifest.json',
        'SHA256SUMS',
        'unins000.exe'
    )
    foreach ($relativePath in $requiredPaths) {
        $path = Join-Path $InstallRoot $relativePath
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Installed application is missing required path: $relativePath"
        }
    }

    $manifest = Get-Content -LiteralPath (Join-Path $InstallRoot 'release-manifest.json') -Raw | ConvertFrom-Json
    if ($manifest.product -ne 'DiskTrace' -or $manifest.target -ne 'windows-x86_64' -or $manifest.primary_launcher -ne 'Start DiskTrace.cmd') {
        throw 'Installed release manifest does not match the Windows distribution contract.'
    }

    foreach ($line in Get-Content -LiteralPath (Join-Path $InstallRoot 'SHA256SUMS')) {
        if ($line -notmatch '^([a-fA-F0-9]{64})  (.+)$') {
            throw "Malformed installed-file checksum: $line"
        }
        $expectedHash = $Matches[1].ToLowerInvariant()
        $relativePath = $Matches[2]
        $path = Join-Path $InstallRoot ($relativePath -replace '/', '\\')
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Installed checksum references a missing path: $relativePath"
        }
        $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        if ($actualHash -ne $expectedHash) {
            throw "Installed-file checksum mismatch: $relativePath"
        }
    }

    $installedCli = Join-Path $InstallRoot 'bin\evidenceforge.exe'
    & $installedCli --help | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Installed CLI returned exit code $LASTEXITCODE for --help."
    }

    $entries = @(Get-InstallEntries $InstallRoot)
    if ($entries.Count -ne 1) {
        throw "Expected one per-user uninstall registration for the disposable install path, found $($entries.Count)."
    }
    if ($entries[0].DisplayName -ne 'DiskTrace Recovery') {
        throw "Installed application has an unexpected uninstall display name: $($entries[0].DisplayName)"
    }

    $UninstallerPath = Join-Path $InstallRoot 'unins000.exe'
    $uninstallArguments = "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /LOG=`"$UninstallLog`""
    $uninstallProcess = Start-Process -FilePath $UninstallerPath -ArgumentList $uninstallArguments -Wait -PassThru
    if ($uninstallProcess.ExitCode -ne 0) {
        throw "Uninstaller returned exit code $($uninstallProcess.ExitCode)."
    }
    if (-not (Test-Path -LiteralPath $UninstallLog -PathType Leaf)) {
        throw 'Uninstaller did not create the requested acceptance log.'
    }
    if (Test-Path -LiteralPath $InstallRoot) {
        throw "Uninstaller left the disposable installation directory in place: $InstallRoot"
    }
    if (@(Get-InstallEntries $InstallRoot).Count -ne 0) {
        throw 'Uninstaller left a per-user uninstall registration for the disposable install path.'
    }

    Write-Output 'Windows installer install/uninstall acceptance verification passed'
}
finally {
    if ($UninstallerPath -and (Test-Path -LiteralPath $UninstallerPath -PathType Leaf)) {
        try {
            Start-Process -FilePath $UninstallerPath -ArgumentList '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART' -Wait | Out-Null
        }
        catch {
            Write-Warning "best-effort installer acceptance cleanup failed: $($_.Exception.Message)"
        }
    }
    Remove-Item -Force -Recurse -ErrorAction SilentlyContinue $Work
}
