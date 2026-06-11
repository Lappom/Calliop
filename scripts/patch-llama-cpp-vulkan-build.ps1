# Patches llama-cpp-sys-2 and whisper-rs-sys for Windows+Vulkan (Ninja + serial jobs).
$ErrorActionPreference = "Stop"

$marker = "calliop-vulkan-win-serial"
$ninjaMarker = "calliop-vulkan-ninja"
$registryRoot = Join-Path $env:USERPROFILE ".cargo\registry\src"

if (-not (Test-Path $registryRoot)) {
    Write-Output "OK"
    exit 0
}

$buildFiles = @()
Get-ChildItem $registryRoot -Directory | ForEach-Object {
    $matches = Get-ChildItem $_.FullName -Filter "llama-cpp-sys-2-*" -Directory -ErrorAction SilentlyContinue
    foreach ($dir in $matches) {
        $buildRs = Join-Path $dir.FullName "build.rs"
        if (Test-Path $buildRs) { $buildFiles += Get-Item $buildRs }
    }
}

$newParallelBlock = @'
    // Speed up build
    // calliop-vulkan-win-serial: MSBuild races on vulkan-shaders-gen ExternalProject
    let cmake_parallel = if cfg!(all(feature = "vulkan", target_os = "windows")) {
        env::set_var("NUM_JOBS", "1");
        "1".to_string()
    } else {
        std::thread::available_parallelism()
            .unwrap()
            .get()
            .to_string()
    };
    env::set_var("CMAKE_BUILD_PARALLEL_LEVEL", cmake_parallel);
'@

$parallelPattern = '(?s)    // Speed up build\r?\n    env::set_var\(\r?\n        "CMAKE_BUILD_PARALLEL_LEVEL",\r?\n        std::thread::available_parallelism\(\)\r?\n            \.unwrap\(\)\r?\n            \.get\(\)\r?\n            \.to_string\(\),\r?\n    \);'
$ninjaPattern = '(?m)^                config\.cxxflag\("/FS"\);\r?$'
$ninjaInsert = @'
                config.cxxflag("/FS");
                // calliop-vulkan-ninja: Ninja avoids MSBuild ExternalProject step races
                config.generator("Ninja");
'@.TrimEnd()
$jobsPattern = '(?m)(let cmake_parallel = if cfg!\(all\(feature = "vulkan", target_os = "windows"\)\) \{\r?\n        )"1\.to_string\(\)'
$jobsReplacement = '${1}env::set_var("NUM_JOBS", "1");' + "`n" + '        "1".to_string()'

$patched = $false
foreach ($file in $buildFiles) {
    $content = [System.IO.File]::ReadAllText($file.FullName)
    $fileChanged = $false

    if ($content -notmatch $marker) {
        if ($content -match $parallelPattern) {
            $content = [regex]::Replace($content, $parallelPattern, $newParallelBlock)
            Write-Host "Patched $($file.Name): serial CMake jobs"
            $fileChanged = $true
        } else {
            Write-Warning "llama-cpp-sys-2 build.rs changed; skip parallel patch: $($file.FullName)"
        }
    } elseif ($content -notmatch 'env::set_var\("NUM_JOBS", "1"\)' -and $content -match $jobsPattern) {
        $content = [regex]::Replace($content, $jobsPattern, $jobsReplacement)
        Write-Host "Updated $($file.Name): NUM_JOBS=1"
        $fileChanged = $true
    }

    if ($content -notmatch $ninjaMarker -and $content -match $ninjaPattern) {
        $content = [regex]::Replace($content, $ninjaPattern, $ninjaInsert)
        Write-Host "Patched $($file.Name): Ninja generator"
        $fileChanged = $true
    }

    if ($fileChanged) {
        [System.IO.File]::WriteAllText($file.FullName, $content)
        $patched = $true
    }
}

$whisperMarker = "calliop-whisper-vulkan-ninja"
$whisperFiles = @()
Get-ChildItem $registryRoot -Directory | ForEach-Object {
    $matches = Get-ChildItem $_.FullName -Filter "whisper-rs-sys-*" -Directory -ErrorAction SilentlyContinue
    foreach ($dir in $matches) {
        $buildRs = Join-Path $dir.FullName "build.rs"
        if (Test-Path $buildRs) { $whisperFiles += Get-Item $buildRs }
    }
}

$whisperOld = '            let vulkan_lib_path = vulkan_path.join("Lib");' + "`n" + '            println!("cargo:rustc-link-search={}", vulkan_lib_path.display());' + "`n" + '        } else if cfg!(target_os = "macos") {'
$whisperNew = @'
            let vulkan_lib_path = vulkan_path.join("Lib");
            println!("cargo:rustc-link-search={}", vulkan_lib_path.display());
            // calliop-whisper-vulkan-ninja: Ninja avoids MSBuild ExternalProject step races
            env::set_var("NUM_JOBS", "1");
            config.generator("Ninja");
        } else if cfg!(target_os = "macos") {
'@.TrimEnd()

foreach ($file in $whisperFiles) {
    $content = [System.IO.File]::ReadAllText($file.FullName)
    if ($content -match $whisperMarker) { continue }
    if (-not $content.Contains($whisperOld)) {
        Write-Warning "whisper-rs-sys build.rs changed; skip patch: $($file.FullName)"
        continue
    }
    $content = $content.Replace($whisperOld, $whisperNew)
    [System.IO.File]::WriteAllText($file.FullName, $content)
    Write-Host "Patched whisper $($file.Name): Ninja generator"
    $patched = $true
}

if ($patched) {
    Write-Output "PATCHED"
} else {
    Write-Output "OK"
}
