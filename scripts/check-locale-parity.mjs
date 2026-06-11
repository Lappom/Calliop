import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

function flattenKeys(obj, prefix = "") {
  const keys = [];
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value !== null && typeof value === "object" && !Array.isArray(value)) {
      keys.push(...flattenKeys(value, path));
    } else {
      keys.push(path);
    }
  }
  return keys.sort();
}

const fr = JSON.parse(readFileSync(join(root, "locales/fr.json"), "utf8"));
const en = JSON.parse(readFileSync(join(root, "locales/en.json"), "utf8"));

const frKeys = flattenKeys(fr);
const enKeys = flattenKeys(en);

const missingInEn = frKeys.filter((key) => !enKeys.includes(key));
const missingInFr = enKeys.filter((key) => !frKeys.includes(key));

if (missingInEn.length || missingInFr.length) {
  if (missingInEn.length) {
    console.error("Missing in en.json:", missingInEn.join(", "));
  }
  if (missingInFr.length) {
    console.error("Missing in fr.json:", missingInFr.join(", "));
  }
  process.exit(1);
}

console.log(`Locale parity OK (${frKeys.length} keys)`);
