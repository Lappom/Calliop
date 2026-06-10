# Dev launcher: ensures CMake is on PATH, then starts Tauri dev server.
$ErrorActionPreference = "Stop"
$repoRoot = Split-Path $PSScriptRoot -Parent

$cmakeExe = & (Join-Path $PSScriptRoot "ensure-cmake.ps1")
$cmakeBin = Split-Path $cmakeExe -Parent
$env:PATH = "$cmakeBin;$env:PATH"

& (Join-Path $PSScriptRoot "prepare-llm-sidecar.ps1")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Set-Location $repoRoot
pnpm tauri dev @args
