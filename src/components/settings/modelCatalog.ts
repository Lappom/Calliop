import type { TFunction } from "i18next";
import type { ModelStatusEntry } from "../../hooks/useSettings";
import type { SelectOption } from "../ui/Select";

export type ModelInstallStatus = "active" | "installed" | "missing";

const WHISPER_MODEL_IDS = [
  "auto",
  "distil-fr-v0.2",
  "distil-fr-dec16",
  "distil-fr-dec16-q8_0",
] as const;
const LLM_MODEL_IDS = ["auto", "qwen3.5-0.8b", "qwen3.5-2b", "qwen3.5-4b"] as const;

const WHISPER_LABEL_KEYS: Record<(typeof WHISPER_MODEL_IDS)[number], string> = {
  auto: "settings.modelsPanel.whisper.auto",
  "distil-fr-v0.2": "settings.modelsPanel.whisper.distilFrV02",
  "distil-fr-dec16": "settings.modelsPanel.whisper.distilFr",
  "distil-fr-dec16-q8_0": "settings.modelsPanel.whisper.distilFrQ8",
};

const LLM_LABEL_KEYS: Record<(typeof LLM_MODEL_IDS)[number], string> = {
  auto: "settings.modelsPanel.llm.auto",
  "qwen3.5-0.8b": "settings.modelsPanel.llm.qwen35_08",
  "qwen3.5-2b": "settings.modelsPanel.llm.qwen35_2",
  "qwen3.5-4b": "settings.modelsPanel.llm.qwen35_4",
};

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

export function getModelStatusLabel(
  status: ModelInstallStatus,
  t: TFunction,
): string {
  switch (status) {
    case "active":
      return t("settings.modelsPanel.status.active");
    case "installed":
      return t("settings.modelsPanel.status.installed");
    default:
      return t("settings.modelsPanel.status.missing");
  }
}

export function getWhisperModelOptions(
  t: TFunction,
): Omit<SelectOption, "status">[] {
  return WHISPER_MODEL_IDS.map((value) => ({
    value,
    label: t(WHISPER_LABEL_KEYS[value]),
  }));
}

export function getLlmModelOptions(t: TFunction): Omit<SelectOption, "status">[] {
  return LLM_MODEL_IDS.map((value) => ({
    value,
    label: t(LLM_LABEL_KEYS[value]),
  }));
}

export function buildWhisperSelectOptions(
  entries: ModelStatusEntry[] | undefined,
  t: TFunction,
): SelectOption[] {
  return getWhisperModelOptions(t).map((option) => {
    const status = resolveModelStatus(entries, option.value);
    return { ...option, status, statusLabel: getModelStatusLabel(status, t) };
  });
}

export function buildLlmSelectOptions(
  entries: ModelStatusEntry[] | undefined,
  t: TFunction,
): SelectOption[] {
  return getLlmModelOptions(t).map((option) => {
    const status = resolveModelStatus(entries, option.value);
    return { ...option, status, statusLabel: getModelStatusLabel(status, t) };
  });
}

export function findModelEntry(
  entries: ModelStatusEntry[] | undefined,
  id: string,
): ModelStatusEntry | undefined {
  return entries?.find((item) => item.id === id);
}

export function listInstalledModels(
  entries: ModelStatusEntry[] | undefined,
): ModelStatusEntry[] {
  return entries?.filter((entry) => entry.installed) ?? [];
}

export function getModelLabel(
  kind: "whisper" | "llm",
  id: string,
  t: TFunction,
): string {
  if (kind === "whisper" && id in WHISPER_LABEL_KEYS) {
    return t(WHISPER_LABEL_KEYS[id as keyof typeof WHISPER_LABEL_KEYS]);
  }
  if (kind === "llm" && id in LLM_LABEL_KEYS) {
    return t(LLM_LABEL_KEYS[id as keyof typeof LLM_LABEL_KEYS]);
  }
  return id;
}
