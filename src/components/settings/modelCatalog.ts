import type { ModelStatusEntry } from "../../hooks/useSettings";
import type { SelectOption } from "../ui/Select";

export type ModelInstallStatus = "active" | "installed" | "missing";

const WHISPER_CATALOG: Omit<SelectOption, "status">[] = [
  { value: "auto", label: "Automatique (recommandé)" },
  { value: "small", label: "Small (~466 Mo)" },
  { value: "distil-fr-dec16", label: "Distil FR dec16 (~755 Mo)" },
];

const LLM_CATALOG: Omit<SelectOption, "status">[] = [
  { value: "auto", label: "Automatique (recommandé)" },
  { value: "qwen3-0.6b", label: "Qwen3 0.6B (~484 Mo)" },
  { value: "qwen3-1.7b", label: "Qwen3 1.7B (~1,1 Go)" },
  { value: "qwen3-4b", label: "Qwen3 4B (~2,5 Go)" },
];

function resolveModelStatus(
  entries: ModelStatusEntry[] | undefined,
  id: string,
): ModelInstallStatus {
  if (id === "auto") {
    return "active";
  }
  const entry = entries?.find((item) => item.id === id);
  if (!entry?.installed) return "missing";
  if (entry.active) return "active";
  return "installed";
}

export function modelStatusLabel(status: ModelInstallStatus): string {
  switch (status) {
    case "active":
      return "Actif";
    case "installed":
      return "Installé";
    default:
      return "Non installé";
  }
}

export function buildWhisperSelectOptions(
  entries: ModelStatusEntry[] | undefined,
): SelectOption[] {
  return WHISPER_CATALOG.map((option) => {
    const status = resolveModelStatus(entries, option.value);
    return { ...option, status, statusLabel: modelStatusLabel(status) };
  });
}

export function buildLlmSelectOptions(
  entries: ModelStatusEntry[] | undefined,
): SelectOption[] {
  return LLM_CATALOG.map((option) => {
    const status = resolveModelStatus(entries, option.value);
    return { ...option, status, statusLabel: modelStatusLabel(status) };
  });
}
