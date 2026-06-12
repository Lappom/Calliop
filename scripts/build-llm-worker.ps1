# Builds calliop-llm-worker with CMake on PATH (portable Calliop install or system cmake).
$ErrorActionPreference = "Stop"
$repoRoot = Split-Path $PSScriptRoot -Parent
$srcTauri = Join-Path $repoRoot "src-tauri"

$cmakeExe = & (Join-Path $PSScriptRoot "ensure-cmake.ps1")
$cmakeBin = Split-Path $cmakeExe -Parent
$env:PATH = "$cmakeBin;$env:PATH"

Push-Location $srcTauri
try {
    cargo build -p calliop-llm-worker @args
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
