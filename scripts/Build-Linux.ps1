[CmdletBinding()]
param([switch]$Development)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    $cargoArguments = @('build', '--locked', '-p', 'puzzgen', '--target', 'x86_64-unknown-linux-musl')
    if (-not $Development) { $cargoArguments += '--release' }
    & cargo @cargoArguments
    if ($LASTEXITCODE -ne 0) { throw 'Linux build failed. Run ./scripts/Setup-Toolchain.ps1 if the target is missing.' }
    $profile = if ($Development) { 'debug' } else { 'release' }
    $binaryPath = Join-Path $projectRoot "target/x86_64-unknown-linux-musl/$profile/puzzgen"
    $header = [System.IO.File]::ReadAllBytes($binaryPath)
    if ($header.Length -lt 20 -or $header[0] -ne 0x7f -or $header[1] -ne 0x45 -or $header[2] -ne 0x4c -or $header[3] -ne 0x46 -or $header[4] -ne 2 -or $header[18] -ne 0x3e) {
        throw 'Build output is not the expected Linux x86-64 ELF executable.'
    }
    Write-Host "Linux x86-64 binary: $binaryPath"
    Write-Host 'This validates the ELF format. Run the binary and smoke tests on your Linux VPS before deployment.'
} finally {
    Pop-Location
}
