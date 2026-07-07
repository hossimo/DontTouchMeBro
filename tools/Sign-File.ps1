<#
.SYNOPSIS
    Authenticode-signs a file with a certificate from the local cert stores.

.DESCRIPTION
    Looks up a code-signing certificate by thumbprint in CurrentUser\My or
    LocalMachine\My and signs the given file (SHA-256 + RFC-3161 timestamp).
    Used by build.ps1 and make-installer.ps1; can also be run directly.

    Note: the signature only shows as "Valid" once the signing certificate is
    trusted (see tools\New-CodeSigningCert.ps1). Signing still succeeds before
    then - Get-AuthenticodeSignature will just report it as untrusted.

.PARAMETER Path
    File to sign (exe/dll/msi/etc.).

.PARAMETER Thumbprint
    Thumbprint of the code-signing certificate.

.PARAMETER TimestampUrl
    RFC-3161 timestamp server. Default: DigiCert.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Path,
    [Parameter(Mandatory)][string]$Thumbprint,
    [string]$TimestampUrl = 'http://timestamp.digicert.com'
)

$ErrorActionPreference = 'Stop'

$thumb = ($Thumbprint -replace '\s', '').ToUpper()
$cert = Get-ChildItem Cert:\CurrentUser\My, Cert:\LocalMachine\My -ErrorAction SilentlyContinue |
    Where-Object { $_.Thumbprint -eq $thumb -and $_.HasPrivateKey } |
    Select-Object -First 1

if (-not $cert) {
    throw "No code-signing certificate with thumbprint '$thumb' (with a private key) found in CurrentUser\My or LocalMachine\My. Run tools\New-CodeSigningCert.ps1 first."
}

$result = Set-AuthenticodeSignature -FilePath $Path -Certificate $cert `
    -TimestampServer $TimestampUrl -HashAlgorithm SHA256

if ($result.Status -eq 'Valid') {
    Write-Host "    Signed (valid): $Path"
} else {
    # Not fatal: signing worked, but the cert chain isn't trusted on this run.
    Write-Warning "Signed but not trusted ($($result.Status)): $Path`n  $($result.StatusMessage)"
}

return $result.Status
