# Ensures CMake 4.x is available for whisper-rs-sys (required on Windows with VS 18).
# Installs a portable copy to %LOCALAPPDATA%\Calliop\tools\cmake if not already present.

$ErrorActionPreference = "Stop"

$cmakeVersion = "4.3.3"
$toolsRoot = Join-Path $env:LOCALAPPDATA "Calliop\tools"
$cmakeRoot = Join-Path $toolsRoot "cmake"
$cmakeExe = Join-Path $cmakeRoot "bin\cmake.exe"

function Test-CmakeUsable {
    param([string]$Path)
    if (-not (Test-Path $Path)) { return $false }
    try {
        $version = & $Path --version 2>&1 | Select-Object -First 1
        return $version -match "cmake version"
    } catch {
        return $false
    }
}

# Check our persistent portable install first.
if (Test-CmakeUsable $cmakeExe) {
    Write-Host "CMake found: $cmakeExe"
    return $cmakeExe
}

# Accept a system CMake on PATH only if it is a real install (not a temp extract).
$systemCmake = Get-Command cmake -ErrorAction SilentlyContinue
if ($systemCmake -and (Test-CmakeUsable $systemCmake.Source)) {
    $src = $systemCmake.Source
    $isTemp = $src -match '\\Temp\\' -or $src -match '/Temp/'
    if (-not $isTemp) {
        Write-Host "CMake found on PATH: $src"
        return $src
    }
}

Write-Host "Installing portable CMake $cmakeVersion to $cmakeRoot ..."
New-Item -ItemType Directory -Force -Path $toolsRoot | Out-Null

$zipUrl = "https://github.com/Kitware/CMake/releases/download/v$cmakeVersion/cmake-$cmakeVersion-windows-x86_64.zip"
$zipPath = Join-Path $env:TEMP "calliop-cmake-$cmakeVersion.zip"
$extractPath = Join-Path $env:TEMP "calliop-cmake-extract"

Invoke-WebRequest -Uri $zipUrl -OutFile $zipPath
if (Test-Path $extractPath) {
    Remove-Item $extractPath -Recurse -Force
}
Expand-Archive -Path $zipPath -DestinationPath $extractPath -Force

$extracted = Get-ChildItem $extractPath -Directory | Select-Object -First 1
if (-not $extracted) {
    throw "CMake archive extraction failed"
}

if (Test-Path $cmakeRoot) {
    Remove-Item $cmakeRoot -Recurse -Force
}
Move-Item $extracted.FullName $cmakeRoot

if (-not (Test-CmakeUsable $cmakeExe)) {
    throw "CMake installation failed at $cmakeExe"
}

Write-Host "CMake installed: $cmakeExe"
return $cmakeExe
