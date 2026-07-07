<#
.SYNOPSIS
    Creates (and optionally trusts) a self-signed code-signing certificate.

.DESCRIPTION
    One-time developer setup for signing local builds. Creates a self-signed
    code-signing cert in CurrentUser\My if one with the given subject doesn't
    already exist, and prints its thumbprint for use with build.ps1
    (-CertThumbprint) / make-installer.ps1.

    With -Trust (requires an ELEVATED shell) it also installs the public
    certificate into the machine's Trusted Root and Trusted Publisher stores so
    signatures validate as "Valid" on this machine.

    IMPORTANT: a self-signed certificate makes signatures *valid* locally, but
    it does NOT establish reputation with Microsoft SmartScreen or Smart App
    Control. SAC may still block the binary. For distribution you need an OV/EV
    certificate from a public CA.

.PARAMETER Subject
    Certificate subject. Default: the company dev-test identity.

.PARAMETER Years
    Validity in years. Default: 3.

.PARAMETER Trust
    Also add the cert to LocalMachine Trusted Root + Trusted Publisher.
    Requires running as Administrator.

.EXAMPLE
    # Non-elevated: just create the cert and print the thumbprint
    .\tools\New-CodeSigningCert.ps1

.EXAMPLE
    # Elevated: create (if needed) and trust it machine-wide
    .\tools\New-CodeSigningCert.ps1 -Trust
#>
[CmdletBinding()]
param(
    [string]$Subject = 'CN=Down Right Technical Inc. (Dev Test)',
    [int]$Years = 3,
    [switch]$Trust
)

$ErrorActionPreference = 'Stop'

# Reuse an existing cert with this subject if present, else create one.
$cert = Get-ChildItem Cert:\CurrentUser\My |
    Where-Object { $_.Subject -eq $Subject -and $_.HasPrivateKey } |
    Sort-Object NotAfter -Descending | Select-Object -First 1

if ($cert) {
    Write-Host "Using existing certificate: $($cert.Thumbprint)"
} else {
    $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject $Subject `
        -CertStoreLocation Cert:\CurrentUser\My -KeyExportPolicy Exportable `
        -KeyUsage DigitalSignature -KeySpec Signature -NotAfter (Get-Date).AddYears($Years)
    Write-Host "Created certificate: $($cert.Thumbprint)"
}

if ($Trust) {
    $isAdmin = ([Security.Principal.WindowsPrincipal] `
        [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
        [Security.Principal.WindowsBuiltinRole]::Administrator)
    if (-not $isAdmin) {
        throw "-Trust requires an elevated (Administrator) PowerShell session."
    }

    # Public cert (no private key) goes into the trust stores.
    $pub = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($cert.Export('Cert'))
    foreach ($storeName in @('Root', 'TrustedPublisher')) {
        $store = [System.Security.Cryptography.X509Certificates.X509Store]::new(
            $storeName, 'LocalMachine')
        $store.Open('ReadWrite')
        $store.Add($pub)
        $store.Close()
        Write-Host "Added to LocalMachine\$storeName"
    }
    Write-Host "Certificate is now trusted on this machine."
}

Write-Host ""
Write-Host "Thumbprint: $($cert.Thumbprint)" -ForegroundColor Green
Write-Host "Use it with:  .\make-installer.ps1 -CertThumbprint $($cert.Thumbprint)"
