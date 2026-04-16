param(
    [switch]$Serve
)

$ErrorActionPreference = 'Stop'

# Ensure script runs from repo root
$root = (Resolve-Path "$PSScriptRoot\.." ).Path
Set-Location $root

Write-Host 'Dev WASM helper: building core-wasm and frontend bundle'

# Repro location for build outputs
$env:CARGO_TARGET_DIR = 'C:\flashcut_target'
$env:RUSTFLAGS = '-C debuginfo=0'

# Ensure wasm target
if (-not (rustup target list | Select-String 'wasm32-unknown-unknown.*installed')) {
    Write-Host 'Adding wasm32-unknown-unknown target'
    rustup target add wasm32-unknown-unknown
}

# Ensure sample video exists (use download helper if missing)
$sample = Join-Path $root 'assets\\sample\\sample.webm'
if (-not (Test-Path $sample)) {
    if (Test-Path "$PSScriptRoot\\download-sample.ps1") {
        Write-Host 'Sample missing — downloading via scripts/download-sample.ps1'
        $downloader = Join-Path $PSScriptRoot 'download-sample.ps1'
        $targetDir = Split-Path $sample -Parent
        if (-not (Test-Path $targetDir)) { New-Item -ItemType Directory -Path $targetDir -Force | Out-Null }
        & $downloader -Out $sample
    }
    else {
        Write-Host 'Sample missing and no downloader present; continuing anyway.'
    }
}

Write-Host 'Building flashcut-core-wasm for wasm32 target...'
cargo build -p flashcut-core-wasm --target wasm32-unknown-unknown

Write-Host 'Building frontend bundle with trunk (no serve)...'
trunk build

if ($Serve) {
    Write-Host 'Starting trunk serve...'
    trunk serve --open
}

Write-Host 'Dev WASM helper completed.'
