import { spawnSync } from "node:child_process";

import { existsSync } from "node:fs";

import { dirname, join } from "node:path";

import { fileURLToPath } from "node:url";



const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

const args = process.argv.slice(2);



function hasGpuInFeaturesValue(value) {

  if (!value) return false;

  return value.split(",").map((f) => f.trim()).includes("gpu");

}



function hasGpuFeatureInArgs(argv) {

  for (let i = 0; i < argv.length; i++) {

    const arg = argv[i];

    if (arg === "--features" && hasGpuInFeaturesValue(argv[i + 1])) {

      return true;

    }

    if (arg.startsWith("--features=") && hasGpuInFeaturesValue(arg.slice("--features=".length))) {

      return true;

    }

  }

  return false;

}



function hasExplicitFeatures(argv) {

  return argv.some((arg) => arg === "--features" || arg.startsWith("--features="));

}



function shouldAutoEnableGpu(command, argv) {

  if (process.platform !== "win32") return false;

  if (hasGpuFeatureInArgs(argv)) return true;

  if ((command === "dev" || command === "build") && !hasExplicitFeatures(argv)) {

    return true;

  }

  return false;

}



function prependPath(dir) {

  if (!dir) return;

  const normalized = dir.toLowerCase();

  if (!process.env.PATH?.toLowerCase().includes(normalized)) {

    process.env.PATH = `${dir};${process.env.PATH ?? ""}`;

  }

}



function runPowerShellScript(scriptName, extraArgs = []) {

  const script = join(repoRoot, "scripts", scriptName);

  return spawnSync(

    "powershell",

    ["-ExecutionPolicy", "Bypass", "-File", script, ...extraArgs],

    { encoding: "utf-8", cwd: repoRoot, env: process.env },

  );

}



function applyVulkanEnv(root) {

  if (!root || !existsSync(join(root, "Lib"))) return false;

  process.env.VULKAN_SDK = root;

  prependPath(join(root, "Bin"));

  return true;

}



function ensureVulkanSdk(installIfMissing) {

  if (process.platform !== "win32") return true;



  const result = runPowerShellScript(

    "ensure-vulkan-sdk.ps1",

    installIfMissing ? ["-InstallIfMissing"] : [],

  );

  if ((result.status ?? 1) !== 0) return false;



  const lines = (result.stdout ?? "")

    .trim()

    .split(/\r?\n/)

    .map((line) => line.trim())

    .filter(Boolean);

  const root = lines.at(-1);

  if (applyVulkanEnv(root)) {

    if (result.stderr) process.stderr.write(result.stderr);

    return true;

  }

  return false;

}



function ensureCmakeOnPath() {

  if (process.platform !== "win32") return true;



  const result = runPowerShellScript("ensure-cmake.ps1");

  if ((result.status ?? 1) !== 0) return false;

  if (result.stderr) process.stderr.write(result.stderr);



  const lines = (result.stdout ?? "")

    .trim()

    .split(/\r?\n/)

    .map((line) => line.trim())

    .filter(Boolean);

  const cmakeExe = lines.at(-1);

  if (!cmakeExe?.toLowerCase().endsWith("cmake.exe")) return false;

  prependPath(dirname(cmakeExe));

  return true;

}



function ensureNinjaOnPath() {

  if (process.platform !== "win32") return;

  if (process.env.PATH?.split(";").some((p) => existsSync(join(p, "ninja.exe")))) return;



  const roots = [

    "C:\\Program Files (x86)\\Microsoft Visual Studio\\18\\BuildTools",

    "C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools",

    "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community",

  ];

  for (const root of roots) {

    const ninjaDir = join(root, "Common7", "IDE", "CommonExtensions", "Microsoft", "CMake", "Ninja");

    if (existsSync(join(ninjaDir, "ninja.exe"))) {

      prependPath(ninjaDir);

      return;

    }

  }

}



function patchVulkanNativeBuilds() {

  if (process.platform !== "win32") return "OK";

  const result = runPowerShellScript("patch-llama-cpp-vulkan-build.ps1");

  if ((result.status ?? 1) !== 0) return "FAIL";

  if (result.stderr) process.stderr.write(result.stderr);

  return (result.stdout ?? "").trim().split(/\r?\n/).filter(Boolean).at(-1) ?? "OK";

}



function configureWindowsNativeBuildEnv(enableGpu) {
  if (process.platform !== "win32") return;

  // whisper-rs-sys Vulkan ExternalProject paths exceed the default 250-char object path in debug.
  if (!process.env.CMAKE_OBJECT_PATH_MAX) {
    process.env.CMAKE_OBJECT_PATH_MAX = "1024";
  }

  if (!process.env.CARGO_TARGET_DIR) {
    process.env.CARGO_TARGET_DIR = join(repoRoot, "ct");
  }

  if (enableGpu) {
    process.env.NUM_JOBS = "1";
  }
}

function ensureWinBuildTools(enableGpu) {

  if (process.platform !== "win32") return true;

  configureWindowsNativeBuildEnv(enableGpu);

  if (!ensureCmakeOnPath()) {

    console.error("[calliop] CMake is required for whisper-rs on Windows. ensure-cmake.ps1 failed.");

    return false;

  }



  if (!enableGpu) return true;

  ensureNinjaOnPath();



  const install = true;

  if (!ensureVulkanSdk(install)) {

    return false;

  }



  patchVulkanNativeBuilds();

  return true;

}



function prepareLlmSidecar(enableGpu) {

  const prepScript =

    process.platform === "win32"

      ? join(repoRoot, "scripts", "prepare-llm-sidecar.ps1")

      : join(repoRoot, "scripts", "prepare-llm-sidecar.sh");



  const env = { ...process.env };

  if (process.platform === "win32" && enableGpu) {

    env.CALLIOP_BUILD_GPU = "1";

  }



  if (process.platform === "win32") {

    return spawnSync(

      "powershell",

      ["-ExecutionPolicy", "Bypass", "-File", prepScript],

      { stdio: "inherit", cwd: repoRoot, env },

    );

  }



  return spawnSync("bash", [prepScript], { stdio: "inherit", cwd: repoRoot, env });

}



function gpuFeatureArgs(command, argv) {

  if (!shouldAutoEnableGpu(command, argv)) return [];

  if (hasGpuFeatureInArgs(argv)) return [];

  if (!ensureVulkanSdk(true)) {

    console.warn(

      "[calliop] Vulkan SDK unavailable — building CPU-only. " +

        "Install: winget install --id KhronosGroup.VulkanSDK",

    );

    return [];

  }

  return ["--features", "gpu"];

}



function runDevWithSidecar(extraArgs) {

  const enableGpu = shouldAutoEnableGpu("dev", extraArgs);

  if (!ensureWinBuildTools(enableGpu)) {

    if (enableGpu) {

      console.warn("[calliop] Dev build continues CPU-only (GPU toolchain unavailable).");

    }

  }



  const prep = prepareLlmSidecar(enableGpu && existsSync(process.env.VULKAN_SDK ?? ""));

  if ((prep.status ?? 1) !== 0) {

    return prep;

  }



  return spawnSync(

    "pnpm",

    ["exec", "tauri", "dev", ...gpuFeatureArgs("dev", extraArgs), ...extraArgs],

    {

      stdio: "inherit",

      cwd: repoRoot,

      shell: true,

      env: process.env,

    },

  );

}



if (args[0] === "dev") {

  const result = runDevWithSidecar(args.slice(1));

  process.exit(result.status ?? 1);

}



if (args[0] === "build") {

  const buildArgs = args.slice(1);

  const enableGpu = shouldAutoEnableGpu("build", buildArgs);

  if (enableGpu) {

    const install = hasGpuFeatureInArgs(buildArgs);

    if (!ensureWinBuildTools(true)) {

      if (!ensureVulkanSdk(install)) {

        console.error(

          "[calliop] GPU build requires LunarG Vulkan SDK (VULKAN_SDK → folder with Lib/ and Bin/).\n" +

            "  1. Open PowerShell as Administrator\n" +

            "  2. winget install --id KhronosGroup.VulkanSDK\n" +

            "  3. Re-run: pnpm tauri build --features gpu\n" +

            "CPU-only: pnpm exec tauri build",

        );

        process.exit(1);

      }

      console.error("[calliop] GPU build toolchain setup failed (CMake/Ninja).");

      process.exit(1);

    }

  } else if (!ensureWinBuildTools(false)) {

    console.error("[calliop] Windows build requires CMake for whisper-rs.");

    process.exit(1);

  }



  const prep = prepareLlmSidecar(enableGpu);

  if ((prep.status ?? 1) !== 0) {

    process.exit(prep.status ?? 1);

  }

}



const command = args[0];

const restArgs = args.slice(1);

const tauriArgs =

  command === "build"

    ? ["build", ...gpuFeatureArgs("build", restArgs), ...restArgs]

    : args;



const result = spawnSync("pnpm", ["exec", "tauri", ...tauriArgs], {

  stdio: "inherit",

  cwd: repoRoot,

  shell: true,

  env: process.env,

});

process.exit(result.status ?? 1);

