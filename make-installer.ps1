<#
.SYNOPSIS
    Builds DontTouchMeBro and compiles the Inno Setup installer in one step.

.DESCRIPTION
    Runs build.ps1 (cargo release build) and then compiles
    installer\DontTouchMeBro.iss with Inno Setup's command-line compiler (ISCC).
    The installer's version is read from the built exe, which gets it from
    Cargo.toml, so the setup filename always matches the app version.

    Finds ISCC.exe automatically in the usual per-user and machine install
    locations, or pass -Iscc to point at it explicitly.

.PARAMETER Configuration
    Build configuration passed to build.ps1 (Release or Debug). Default: Release.

.PARAMETER SkipBuild
    Skip the build step and just compile the installer against whatever is
    already in dist\win-x64.

.PARAMETER Iscc
    Full path to ISCC.exe. Overrides auto-detection.

.EXAMPLE
    .\make-installer.ps1
    Build + compile; installer written to installer\Output.

.EXAMPLE
    .\make-installer.ps1 -SkipBuild
    Recompile the installer without rebuilding.
#>
[CmdletBinding()]
param(
    [string]$Configuration = 'Release',
    [switch]$SkipBuild,
    [string]$Iscc          = ''
)

$ErrorActionPreference = 'Stop'

$repoRoot  = $PSScriptRoot
$buildPs1  = Join-Path $repoRoot 'build.ps1'
$issScript = Join-Path $repoRoot 'installer\DontTouchMeBro.iss'

# --- Locate ISCC.exe -------------------------------------------------------
function Find-Iscc {
    param([string]$Override)

    if ($Override) {
        if (Test-Path $Override) { return $Override }
        throw "ISCC not found at the path passed via -Iscc: $Override"
    }

    # Any "Inno Setup <major>" folder, newest first, in the per-user and
    # machine install locations.
    $roots = @("$env:LOCALAPPDATA\Programs", ${env:ProgramFiles(x86)}, $env:ProgramFiles) |
        Where-Object { $_ -and (Test-Path $_) }
    $hit = $roots |
        ForEach-Object { Get-ChildItem -Path $_ -Directory -Filter 'Inno Setup*' -ErrorAction SilentlyContinue } |
        Sort-Object Name -Descending |
        ForEach-Object { Join-Path $_.FullName 'ISCC.exe' } |
        Where-Object { Test-Path $_ } |
        Select-Object -First 1
    if ($hit) { return $hit }

    $cmd = Get-Command iscc -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    throw "Inno Setup (ISCC.exe) not found. Install it (e.g. 'winget install JRSoftware.InnoSetup') or pass -Iscc <path>."
}

$isccPath = Find-Iscc -Override $Iscc
Write-Host "==> Using ISCC: $isccPath" -ForegroundColor Cyan

# --- Build -----------------------------------------------------------------
if (-not $SkipBuild) {
    # build.ps1 throws on failure, which stops this script too.
    & $buildPs1 -Configuration $Configuration
} else {
    Write-Host "==> Skipping build (-SkipBuild)" -ForegroundColor Yellow
}

# --- Compile installer -----------------------------------------------------
Write-Host "==> Compiling installer: $issScript" -ForegroundColor Cyan
& $isccPath $issScript
if ($LASTEXITCODE -ne 0) { throw "ISCC failed with exit code $LASTEXITCODE" }

# --- Report ----------------------------------------------------------------
$outputDir = Join-Path $repoRoot 'installer\Output'
$setup = Get-ChildItem -Path $outputDir -Filter 'DontTouchMeBro-Setup-*.exe' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1

if ($setup) {
    $sizeMb = [math]::Round($setup.Length / 1MB, 1)
    Write-Host ""
    Write-Host "==> Installer ready" -ForegroundColor Green
    Write-Host "    $($setup.FullName) ($sizeMb MB)"
} else {
    throw "ISCC reported success but no setup exe was found in $outputDir."
}
