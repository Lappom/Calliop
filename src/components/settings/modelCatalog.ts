import type { TFunction } from "i18next";
import type { ModelStatusEntry } from "../../hooks/useSettings";
import type { SelectOption } from "../ui/Select";

export type ModelInstallStatus = "active" | "installed" | "missing";

const WHISPER_MODEL_IDS = ["auto", "small", "distil-fr-dec16"] as const;
const LLM_MODEL_IDS = ["auto", "qwen3-0.6b", "qwen3-1.7b", "qwen3-4b"] as const;

const WHISPER_LABEL_KEYS: Record<(typeof WHISPER_MODEL_IDS)[number], string> = {
  auto: "settings.modelsPanel.whisper.auto",
  small: "settings.modelsPanel.whisper.small",
  "distil-fr-dec16": "settings.modelsPanel.whisper.distilFr",
};

const LLM_LABEL_KEYS: Record<(typeof LLM_MODEL_IDS)[number], string> = {
  auto: "settings.modelsPanel.llm.auto",
  "qwen3-0.6b": "settings.modelsPanel.llm.qwen06",
  "qwen3-1.7b": "settings.modelsPanel.llm.qwen17",
  "qwen3-4b": "settings.modelsPanel.llm.qwen4",
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
