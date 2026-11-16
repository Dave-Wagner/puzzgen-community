[CmdletBinding()]
param([switch]$SkipFrontendTools)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    throw 'Install Rust using https://rustup.rs/ first, then restart PowerShell.'
}

# rustup stores targets in the existing user toolchain. No administrator rights
# or machine-wide PATH changes are needed.
& rustup target add x86_64-unknown-linux-musl wasm32-unknown-unknown
if ($LASTEXITCODE -ne 0) { throw 'Rust target installation failed.' }

if (-not $SkipFrontendTools) {
    if ($env:OS -ne 'Windows_NT') {
        throw 'The project-local Trunk installer supports Windows x64. On Linux use cargo install --locked --version 0.22.1 trunk.'
    }
    if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() -ne 'X64') {
        throw 'This installer pins the Windows x64 Trunk binary. Use cargo install --locked --version 0.22.1 trunk on other architectures.'
    }
    $trunkVersion = '0.22.1'
    $trunkDirectory = Join-Path $projectRoot '.tools/trunk'
    $trunkExecutable = Join-Path $trunkDirectory 'trunk.exe'
    $expectedChecksum = '1cc65012a9df60b02db948f5a3f0601e594ce952efb1cfd23147c9021c9f7d75'
    $installedVersion = if (Test-Path -LiteralPath $trunkExecutable) { & $trunkExecutable --version } else { '' }
    if ($installedVersion -ne "trunk $trunkVersion") {
        New-Item -ItemType Directory -Force -Path $trunkDirectory | Out-Null
        $archivePath = Join-Path $trunkDirectory 'archive.zip'
        $releaseUrl = "https://github.com/trunk-rs/trunk/releases/download/v$trunkVersion/trunk-x86_64-pc-windows-msvc.zip"
        Invoke-WebRequest -Uri $releaseUrl -OutFile $archivePath
        $actualChecksum = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash
        if ($actualChecksum -ne $expectedChecksum) {
            throw 'Trunk archive checksum mismatch; the archive was not extracted.'
        }
        Expand-Archive -LiteralPath $archivePath -DestinationPath $trunkDirectory -Force
    }
    & $trunkExecutable --version
    if ($LASTEXITCODE -ne 0) { throw 'Trunk installation did not complete.' }
}

Write-Host 'Toolchain ready. Linux builds use the bundled rust-lld linker; no WSL or Docker is required.'
Write-Host 'Run ./scripts/Build-Linux.ps1 and ./scripts/Build-Frontend.ps1 from this checkout.'
