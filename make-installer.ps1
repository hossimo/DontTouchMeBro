<#
.SYNOPSIS
    Publishes DontTouchMeBro and compiles the Inno Setup installer in one step.

.DESCRIPTION
    Runs build.ps1 (self-contained single-file publish) and then compiles
    installer\DontTouchMeBro.iss with Inno Setup's command-line compiler (ISCC).
    The installer's version is read from the published exe, so passing -Version
    here flows all the way through to the setup filename.

    Finds ISCC.exe automatically in the usual per-user and machine install
    locations, or pass -Iscc to point at it explicitly.

.PARAMETER Configuration
    MSBuild configuration passed to build.ps1. Default: Release.

.PARAMETER Version
    Optional version override (e.g. 1.1.0.0) passed to build.ps1. When omitted,
    the version from Properties/AssemblyInfo.cs is used.

.PARAMETER SkipBuild
    Skip the publish step and just compile the installer against whatever is
    already in dist\win-x64.

.PARAMETER Iscc
    Full path to ISCC.exe. Overrides auto-detection.

.EXAMPLE
    .\make-installer.ps1
    Publish + compile; installer written to installer\Output.

.EXAMPLE
    .\make-installer.ps1 -Version 1.1.0.0
    Stamp the build as 1.1.0.0 and produce DontTouchMeBro-Setup-1.1.0.0.exe.

.EXAMPLE
    .\make-installer.ps1 -SkipBuild
    Recompile the installer without republishing.
#>
[CmdletBinding()]
param(
    [string]$Configuration = 'Release',
    [string]$Version       = '',
    [switch]$SkipBuild,
    [string]$Iscc          = '',
    [string]$CertThumbprint = '',
    [string]$TimestampUrl   = 'http://timestamp.digicert.com'
)

$ErrorActionPreference = 'Stop'

$repoRoot  = $PSScriptRoot
$buildPs1  = Join-Path $repoRoot 'build.ps1'
$issScript = Join-Path $repoRoot 'installer\DontTouchMeBro.iss'
$signFile  = Join-Path $repoRoot 'tools\Sign-File.ps1'

# --- Locate ISCC.exe -------------------------------------------------------
function Find-Iscc {
    param([string]$Override)

    if ($Override) {
        if (Test-Path $Override) { return $Override }
        throw "ISCC not found at the path passed via -Iscc: $Override"
    }

    $candidates = @(
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
    )
    $hit = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if ($hit) { return $hit }

    $cmd = Get-Command iscc -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    throw "Inno Setup (ISCC.exe) not found. Install it (e.g. 'winget install JRSoftware.InnoSetup') or pass -Iscc <path>."
}

$isccPath = Find-Iscc -Override $Iscc
Write-Host "==> Using ISCC: $isccPath" -ForegroundColor Cyan

# --- Publish (signs the exe too when -CertThumbprint is given) --------------
if (-not $SkipBuild) {
    $buildArgs = @{ Configuration = $Configuration }
    if ($Version) { $buildArgs['Version'] = $Version }
    if ($CertThumbprint) {
        $buildArgs['CertThumbprint'] = $CertThumbprint
        $buildArgs['TimestampUrl']   = $TimestampUrl
    }
    & $buildPs1 @buildArgs
    if ($LASTEXITCODE -ne 0) { throw "build.ps1 failed with exit code $LASTEXITCODE" }
} else {
    Write-Host "==> Skipping publish (-SkipBuild)" -ForegroundColor Yellow
}

# --- Compile installer -----------------------------------------------------
Write-Host "==> Compiling installer: $issScript" -ForegroundColor Cyan
& $isccPath $issScript
if ($LASTEXITCODE -ne 0) { throw "ISCC failed with exit code $LASTEXITCODE" }

# --- Report ----------------------------------------------------------------
$outputDir = Join-Path $repoRoot 'installer\Output'
$setup = Get-ChildItem -Path $outputDir -Filter 'DontTouchMeBro-Setup-*.exe' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1

if (-not $setup) {
    throw "ISCC reported success but no setup exe was found in $outputDir."
}

# Sign the installer itself so the setup exe isn't flagged when distributed.
if ($CertThumbprint) {
    Write-Host "==> Signing installer..." -ForegroundColor Cyan
    & $signFile -Path $setup.FullName -Thumbprint $CertThumbprint -TimestampUrl $TimestampUrl
}

$sizeMb = [math]::Round($setup.Length / 1MB, 1)
Write-Host ""
Write-Host "==> Installer ready" -ForegroundColor Green
Write-Host "    $($setup.FullName) ($sizeMb MB)"
