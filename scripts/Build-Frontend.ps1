[CmdletBinding()]
param([switch]$Development)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$localTrunk = Join-Path $projectRoot '.tools/trunk/trunk.exe'
$trunkCommand = if (Test-Path -LiteralPath $localTrunk) { $localTrunk } else { 'trunk' }
if (-not (Get-Command $trunkCommand -ErrorAction SilentlyContinue)) {
    throw 'Trunk is missing. Run ./scripts/Setup-Toolchain.ps1 first.'
}
Push-Location (Join-Path $projectRoot 'frontend')
try {
    $trunkArguments = @('build', '--locked', '--skip-version-check')
    if (-not $Development) { $trunkArguments += '--release' }
    & $trunkCommand @trunkArguments
    if ($LASTEXITCODE -ne 0) { throw 'Frontend build failed.' }
    Write-Host "Frontend assets: $(Join-Path $projectRoot 'frontend/dist')"
} finally {
    Pop-Location
}
