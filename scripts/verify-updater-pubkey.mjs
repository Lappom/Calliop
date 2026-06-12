import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const configPath = path.join(root, "src-tauri", "tauri.conf.json");
const pubkeyPath = path.join(root, "src-tauri", ".tauri", "calliop.key.pub");

const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
const configPubkey = config.plugins?.updater?.pubkey?.trim();
const filePubkey = fs.readFileSync(pubkeyPath, "utf8").trim();

if (!configPubkey) {
  console.error("Missing plugins.updater.pubkey in src-tauri/tauri.conf.json");
  process.exit(1);
}

if (configPubkey !== filePubkey) {
  console.error(
    "Updater pubkey mismatch: tauri.conf.json must match src-tauri/.tauri/calliop.key.pub",
  );
  process.exit(1);
}

console.log("Updater pubkey matches calliop.key.pub");
