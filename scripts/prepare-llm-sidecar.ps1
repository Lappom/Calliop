# Builds the LLM sidecar and copies it to src-tauri/bin for tauri-build.
$ErrorActionPreference = "Stop"
$repoRoot = Split-Path $PSScriptRoot -Parent
$srcTauri = Join-Path $repoRoot "src-tauri"
$binDir = Join-Path $srcTauri "bin"

$cmakeExe = & (Join-Path $PSScriptRoot "ensure-cmake.ps1")
$cmakeBin = Split-Path $cmakeExe -Parent
$env:PATH = "$cmakeBin;$env:PATH"

Push-Location $srcTauri
try {
    cargo build --features llm-worker --bin calliop-llm-worker
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $triple = & rustc --print host-tuple
    $builtWorker = Join-Path $srcTauri "target/debug/calliop-llm-worker.exe"
    if (-not (Test-Path $builtWorker)) {
        $builtWorker = Join-Path $srcTauri "target/debug/calliop-llm-worker"
    }

    New-Item -ItemType Directory -Force -Path $binDir | Out-Null
    $dest = Join-Path $binDir "calliop-llm-worker-$triple.exe"
    if (-not (Test-Path $builtWorker) -and (Test-Path ($builtWorker -replace '\.exe$', ''))) {
        $dest = Join-Path $binDir "calliop-llm-worker-$triple"
        $builtWorker = $builtWorker -replace '\.exe$', ''
    }

    Copy-Item -Force $builtWorker $dest
    Write-Host "LLM sidecar ready: $dest"
} finally {
    Pop-Location
}
