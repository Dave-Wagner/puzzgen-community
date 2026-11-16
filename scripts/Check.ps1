[CmdletBinding()]
param([ValidateSet('All', 'Backend', 'Frontend', 'Game', 'Native')][string]$Scope = 'All')

# Focused local checks require no AI inference and no hosted CI jobs.
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
function Invoke-CargoCheck {
    param([string[]]$CargoArguments)
    & cargo @CargoArguments
    if ($LASTEXITCODE -ne 0) { throw "Check failed: cargo $($CargoArguments -join ' ')" }
}
Push-Location $projectRoot
try {
    Invoke-CargoCheck -CargoArguments @('fmt', '--all', '--check')
    if ($Scope -in @('All', 'Backend')) {
        Invoke-CargoCheck -CargoArguments @('test', '--locked', '-p', 'puzzgen')
        Invoke-CargoCheck -CargoArguments @('clippy', '--locked', '-p', 'puzzgen', '--all-targets', '--', '-D', 'warnings')
    }
    if ($Scope -in @('All', 'Frontend')) {
        Invoke-CargoCheck -CargoArguments @('clippy', '--locked', '-p', 'puzzgen-web', '--target', 'wasm32-unknown-unknown', '--', '-D', 'warnings')
    }
    if ($Scope -in @('All', 'Game')) {
        Invoke-CargoCheck -CargoArguments @('test', '--locked', '-p', 'puzzgen-game')
        Invoke-CargoCheck -CargoArguments @('clippy', '--locked', '-p', 'puzzgen-game', '--target', 'wasm32-unknown-unknown', '--', '-D', 'warnings')
    }
    if ($Scope -in @('All', 'Native')) {
        Invoke-CargoCheck -CargoArguments @('clippy', '--locked', '-p', 'puzzgen-native', '--lib', '--', '-D', 'warnings')
    }
    & git diff --check
    if ($LASTEXITCODE -ne 0) { throw 'Git whitespace check failed.' }
} finally { Pop-Location }
