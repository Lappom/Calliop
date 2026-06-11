# Ensures LunarG Vulkan SDK is available for GPU builds (llama-cpp / whisper-rs).
# Sets $env:VULKAN_SDK and prepends SDK Bin to PATH for the current process.

param(
    [switch]$InstallIfMissing
)

$ErrorActionPreference = "Stop"

# Keep in sync with winget KhronosGroup.VulkanSDK.
$VulkanSdkVersion = "1.4.350.0"

function Test-IsAdmin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]$identity
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Test-VulkanSdkRoot {
    param([string]$Root)
    if (-not $Root) { return $false }
    $lib = Join-Path $Root "Lib"
    $bin = Join-Path $Root "Bin"
    return (Test-Path $lib) -and (Test-Path $bin)
}

function Find-VulkanSdkRoot {
    if ($env:VULKAN_SDK -and (Test-VulkanSdkRoot $env:VULKAN_SDK)) {
        return $env:VULKAN_SDK
    }

    $candidates = @()
    $vulkanRoot = "C:\VulkanSDK"
    if (Test-Path $vulkanRoot) {
        $candidates += Get-ChildItem $vulkanRoot -Directory -ErrorAction SilentlyContinue |
            Sort-Object Name -Descending |
            ForEach-Object { $_.FullName }
    }

    foreach ($path in $candidates) {
        if (Test-VulkanSdkRoot $path) {
            return $path
        }
    }

    return $null
}

function Install-VulkanSdkSilent {
    param([string]$Version)

    if (-not (Test-IsAdmin)) {
        throw @"
Silent Vulkan SDK install requires an elevated PowerShell (Run as administrator).

Then run:
  powershell -ExecutionPolicy Bypass -File "$PSScriptRoot\ensure-vulkan-sdk.ps1" -InstallIfMissing
"@
    }

    $installer = Join-Path $env:TEMP "vulkansdk-windows-X64-$Version.exe"
    $root = "C:\VulkanSDK\$Version"
    $url = "https://sdk.lunarg.com/sdk/download/$Version/windows/vulkansdk-windows-X64-$Version.exe"

    if (-not (Test-Path $installer)) {
        Write-Host "Downloading Vulkan SDK $Version..."
        Invoke-WebRequest -Uri $url -OutFile $installer
    }

    Write-Host "Installing Vulkan SDK silently to $root ..."
    $args = @(
        "--root", $root,
        "--accept-licenses",
        "--default-answer",
        "--confirm-command", "install"
    )
    $proc = Start-Process -FilePath $installer -ArgumentList $args -Wait -PassThru
    if ($proc.ExitCode -ne 0) {
        throw "Vulkan SDK installer exited with code $($proc.ExitCode)"
    }

    if (-not (Test-VulkanSdkRoot $root)) {
        throw "Vulkan SDK installer finished but $root is missing Lib/ or Bin/."
    }

    return $root
}

function Install-VulkanSdk {
    Write-Host "Trying winget (KhronosGroup.VulkanSDK)..."
    winget install --id KhronosGroup.VulkanSDK --accept-package-agreements --accept-source-agreements
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "winget install failed (exit $LASTEXITCODE); falling back to silent installer."
    } else {
        foreach ($wait in 5, 10, 15, 30) {
            Start-Sleep -Seconds $wait
            $found = Find-VulkanSdkRoot
            if ($found) { return $found }
        }
        Write-Warning "winget reported success but C:\VulkanSDK\<version> was not created."
    }

    return Install-VulkanSdkSilent -Version $VulkanSdkVersion
}

$root = Find-VulkanSdkRoot
if (-not $root -and $InstallIfMissing) {
    $root = Install-VulkanSdk
}

if (-not $root) {
    Write-Error @"
Vulkan SDK not found. GPU builds require LunarG Vulkan SDK with VULKAN_SDK set.

Run in an elevated PowerShell:
  powershell -ExecutionPolicy Bypass -File "$PSScriptRoot\ensure-vulkan-sdk.ps1" -InstallIfMissing

Or download from https://vulkan.lunarg.com/sdk/home (default: C:\VulkanSDK\<version>).

CPU-only build: omit --features gpu
"@
    exit 1
}

$env:VULKAN_SDK = $root
$bin = Join-Path $root "Bin"
if ($env:PATH -notlike "*$bin*") {
    $env:PATH = "$bin;$env:PATH"
}

[Console]::Error.WriteLine("Vulkan SDK: $root")
Write-Output $root
