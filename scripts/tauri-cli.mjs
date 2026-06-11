import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);

function prepareLlmSidecar() {
  const prepScript =
    process.platform === "win32"
      ? join(repoRoot, "scripts", "prepare-llm-sidecar.ps1")
      : join(repoRoot, "scripts", "prepare-llm-sidecar.sh");

  if (process.platform === "win32") {
    return spawnSync(
      "powershell",
      ["-ExecutionPolicy", "Bypass", "-File", prepScript],
      { stdio: "inherit", cwd: repoRoot },
    );
  }

  return spawnSync("bash", [prepScript], { stdio: "inherit", cwd: repoRoot });
}

function runDevWithSidecar(extraArgs) {
  const prep = prepareLlmSidecar();
  if ((prep.status ?? 1) !== 0) {
    return prep;
  }

  return spawnSync("pnpm", ["exec", "tauri", "dev", ...extraArgs], {
    stdio: "inherit",
    cwd: repoRoot,
    shell: true,
  });
}

if (args[0] === "dev") {
  const result = runDevWithSidecar(args.slice(1));
  process.exit(result.status ?? 1);
}

if (args[0] === "build") {
  const prep = prepareLlmSidecar();
  if ((prep.status ?? 1) !== 0) {
    process.exit(prep.status ?? 1);
  }
}

const result = spawnSync("pnpm", ["exec", "tauri", ...args], {
  stdio: "inherit",
  cwd: repoRoot,
  shell: true,
});
process.exit(result.status ?? 1);
