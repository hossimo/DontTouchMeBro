<#
.SYNOPSIS
    Builds and publishes DontTouchMeBro into a distributable folder.

.DESCRIPTION
    Wraps `dotnet publish` with sensible defaults for shipping this WinForms
    tray app. By default it produces a self-contained, single-file x64 build so
    end users do NOT need the .NET runtime installed - which is what the
    (future) per-user installer will package.

    The dev-only device-id.txt is stripped from the output so a build never
    ships someone's personal device configuration.

.PARAMETER Configuration
    MSBuild configuration. Default: Release.

.PARAMETER Runtime
    Runtime identifier to publish for. Default: win-x64.

.PARAMETER SelfContained
    Bundle the .NET runtime so no install is required. Default: $true.
    Pass -SelfContained:$false for a smaller, framework-dependent build.

.PARAMETER SingleFile
    Emit a single packed .exe. Default: $true.

.PARAMETER Version
    Optional version override (e.g. 1.2.3.0). When omitted, the version from
    Properties/AssemblyInfo.cs is used.

.PARAMETER Output
    Output directory. Default: <repo>\dist\<Runtime>.

.EXAMPLE
    .\build.ps1
    Self-contained single-file Release build in .\dist\win-x64.

.EXAMPLE
    .\build.ps1 -SelfContained:$false -Version 1.1.0.0
    Framework-dependent build stamped as 1.1.0.0.
#>
[CmdletBinding()]
param(
    [string]$Configuration = 'Release',
    [string]$Runtime       = 'win-x64',
    [bool]  $SelfContained = $true,
    [bool]  $SingleFile    = $true,
    [string]$Version       = '',
    [string]$Output        = ''
)

$ErrorActionPreference = 'Stop'

$repoRoot = $PSScriptRoot
$project  = Join-Path $repoRoot 'DontTouchMeBro\DontTouchMeBro.csproj'

if (-not $Output) {
    $Output = Join-Path $repoRoot "dist\$Runtime"
}

Write-Host "==> Publishing DontTouchMeBro" -ForegroundColor Cyan
Write-Host "    Configuration : $Configuration"
Write-Host "    Runtime       : $Runtime"
Write-Host "    SelfContained : $SelfContained"
Write-Host "    SingleFile    : $SingleFile"
Write-Host "    Output        : $Output"

# Start from a clean output directory so stale files never linger.
if (Test-Path $Output) {
    try {
        Remove-Item -Recurse -Force $Output -ErrorAction Stop
    } catch {
        throw "Could not clean '$Output' - is DontTouchMeBro.exe still running? " +
              "Close the app (it lives in the tray) and try again.`n$($_.Exception.Message)"
    }
}

$publishArgs = @(
    'publish', $project,
    '-c', $Configuration,
    '-r', $Runtime,
    "--self-contained=$($SelfContained.ToString().ToLower())",
    "-p:PublishSingleFile=$($SingleFile.ToString().ToLower())",
    '-o', $Output,
    '--nologo'
)

if ($Version) {
    $publishArgs += "-p:Version=$Version"
}

& dotnet @publishArgs
if ($LASTEXITCODE -ne 0) {
    throw "dotnet publish failed with exit code $LASTEXITCODE"
}

# Never ship a developer's personal device configuration.
$strayConfig = Join-Path $Output 'device-id.txt'
if (Test-Path $strayConfig) {
    Remove-Item -Force $strayConfig
    Write-Host "    Removed dev device-id.txt from output"
}

$exe = Join-Path $Output 'DontTouchMeBro.exe'
if (Test-Path $exe) {
    $sizeMb = [math]::Round((Get-Item $exe).Length / 1MB, 1)
    Write-Host ""
    Write-Host "==> Build succeeded" -ForegroundColor Green
    Write-Host "    Executable : $exe ($sizeMb MB)"
} else {
    throw "Publish completed but $exe was not found."
}
