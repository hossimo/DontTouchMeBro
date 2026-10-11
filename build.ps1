<#
.SYNOPSIS
    Builds DontTouchMeBro and copies the exe into a distributable folder.

.DESCRIPTION
    Wraps `cargo build` for shipping this tray app. The result is a single,
    self-contained DontTouchMeBro.exe (no runtime to install), which is what
    the installer (installer\DontTouchMeBro.iss) packages.

    The version comes from Cargo.toml.

.PARAMETER Configuration
    Release (default) or Debug.

.PARAMETER Output
    Output directory. Default: <repo>\dist\win-x64.

.EXAMPLE
    .\build.ps1
    Release build in .\dist\win-x64.
#>
[CmdletBinding()]
param(
    [ValidateSet('Release', 'Debug')]
    [string]$Configuration = 'Release',
    [string]$Output        = ''
)

$ErrorActionPreference = 'Stop'

$repoRoot = $PSScriptRoot
if (-not $Output) {
    $Output = Join-Path $repoRoot 'dist\win-x64'
}

Write-Host "==> Building DontTouchMeBro" -ForegroundColor Cyan
Write-Host "    Configuration : $Configuration"
Write-Host "    Output        : $Output"

$cargoArgs = @('build', '--locked', '--manifest-path', (Join-Path $repoRoot 'Cargo.toml'))
$profileDir = 'debug'
if ($Configuration -eq 'Release') {
    $cargoArgs += '--release'
    $profileDir = 'release'
}

& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
}

# Start from a clean output directory so stale files never linger.
if (Test-Path $Output) {
    try {
        Remove-Item -Recurse -Force $Output -ErrorAction Stop
    } catch {
        throw "Could not clean '$Output' - is DontTouchMeBro.exe still running? " +
              "Close the app (it lives in the tray) and try again.`n$($_.Exception.Message)"
    }
}
New-Item -ItemType Directory -Force $Output | Out-Null

$built = Join-Path $repoRoot "target\$profileDir\DontTouchMeBro.exe"
$exe = Join-Path $Output 'DontTouchMeBro.exe'
Copy-Item $built $exe

$sizeMb = [math]::Round((Get-Item $exe).Length / 1MB, 1)
$version = (Get-Item $exe).VersionInfo.FileVersion
Write-Host ""
Write-Host "==> Build succeeded" -ForegroundColor Green
Write-Host "    Executable : $exe ($sizeMb MB, version $version)"
