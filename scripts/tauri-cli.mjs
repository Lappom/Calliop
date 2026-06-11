import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);

function runDevWithSidecar(extraArgs) {
  const prepScript =
    process.platform === "win32"
      ? join(repoRoot, "scripts", "prepare-llm-sidecar.ps1")
      : join(repoRoot, "scripts", "prepare-llm-sidecar.sh");

  if (process.platform === "win32") {
    const prep = spawnSync(
      "powershell",
      ["-ExecutionPolicy", "Bypass", "-File", prepScript],
      { stdio: "inherit", cwd: repoRoot },
    );
    if ((prep.status ?? 1) !== 0) {
      return prep;
    }
  } else {
    const prep = spawnSync("bash", [prepScript], { stdio: "inherit", cwd: repoRoot });
    if ((prep.status ?? 1) !== 0) {
      return prep;
    }
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

const result = spawnSync("pnpm", ["exec", "tauri", ...args], {
  stdio: "inherit",
  cwd: repoRoot,
  shell: true,
});
process.exit(result.status ?? 1);
