# Builds the LLM sidecar and copies it to src-tauri/bin for tauri-build.
$ErrorActionPreference = "Stop"
$repoRoot = Split-Path $PSScriptRoot -Parent
$srcTauri = Join-Path $repoRoot "src-tauri"
$binDir = Join-Path $srcTauri "bin"
$cargoTargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $srcTauri "target" }

if (-not $env:CMAKE_OBJECT_PATH_MAX) {
    $env:CMAKE_OBJECT_PATH_MAX = "1024"
}

$cmakeExe = & (Join-Path $PSScriptRoot "ensure-cmake.ps1")
$cmakeBin = Split-Path $cmakeExe -Parent
$env:PATH = "$cmakeBin;$env:PATH"

function Add-NinjaToPath {
    if (Get-Command ninja -ErrorAction SilentlyContinue) { return }
    $roots = @(
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\18\BuildTools",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Enterprise",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Community"
    )
    foreach ($root in $roots) {
        $ninjaDir = Join-Path $root "Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja"
        if (Test-Path (Join-Path $ninjaDir "ninja.exe")) {
            $env:PATH = "$ninjaDir;$env:PATH"
            return
        }
    }
    Write-Warning "ninja.exe not found; install Visual Studio CMake/Ninja components for GPU builds"
}

$buildGpu = ($env:CALLIOP_BUILD_GPU -eq "1") -or ($env:VULKAN_SDK -and (Test-Path (Join-Path $env:VULKAN_SDK "Lib")))

if ($buildGpu) {
    Add-NinjaToPath
    $null = & (Join-Path $PSScriptRoot "ensure-vulkan-sdk.ps1") -InstallIfMissing
    # Cargo sets NUM_JOBS for build scripts; cmake-rs forwards it as `cmake --build --parallel N`,
    # which races MSBuild on vulkan-shaders-gen ExternalProject steps.
    $env:NUM_JOBS = "1"
    $workerManifest = Join-Path $srcTauri "crates\calliop-llm-worker\Cargo.toml"
    cargo fetch --manifest-path $workerManifest --features gpu
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $patchResult = & (Join-Path $PSScriptRoot "patch-llama-cpp-vulkan-build.ps1") | Select-Object -Last 1
    if ($patchResult -eq "PATCHED" -or $env:CALLIOP_CLEAN_LLAMA_VULKAN -eq "1") {
        foreach ($profile in @("release", "debug")) {
            $buildRoot = Join-Path $cargoTargetDir "$profile\build"
            if (Test-Path $buildRoot) {
                Get-ChildItem $buildRoot -Filter "llama-cpp-sys-2-*" -Directory -ErrorAction SilentlyContinue |
                    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
                Get-ChildItem $buildRoot -Filter "whisper-rs-sys-*" -Directory -ErrorAction SilentlyContinue |
                    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }
}

Push-Location $srcTauri
try {
    if ($buildGpu) {
        Write-Host "Building LLM sidecar with GPU (Vulkan)..."
        cargo build --release -p calliop-llm-worker --features gpu
    } else {
        cargo build --release -p calliop-llm-worker
    }
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $triple = & rustc --print host-tuple
    $builtWorker = Join-Path $cargoTargetDir "release/calliop-llm-worker.exe"
    if (-not (Test-Path $builtWorker)) {
        $builtWorker = Join-Path $cargoTargetDir "release/calliop-llm-worker"
    }

    New-Item -ItemType Directory -Force -Path $binDir | Out-Null
    $dest = Join-Path $binDir "calliop-llm-worker-$triple.exe"
    if (-not (Test-Path $builtWorker) -and (Test-Path ($builtWorker -replace '\.exe$', ''))) {
        $dest = Join-Path $binDir "calliop-llm-worker-$triple"
        $builtWorker = $builtWorker -replace '\.exe$', ''
    }

    $staging = "$dest.tmp"
    Copy-Item -Force $builtWorker $staging
    Move-Item -Force $staging $dest
    Write-Host "LLM sidecar ready: $dest"
} finally {
    Pop-Location
}
