import type { TFunction } from "i18next";
import type { ModelDownloadKind } from "../hooks/useModelDownloads";

const MODEL_LABEL_KEYS: Record<string, string> = {
  "distil-fr-v0.2": "settings.modelsPanel.whisper.distilFrV02",
  "distil-fr-dec16": "settings.modelsPanel.whisper.distilFr",
  "distil-fr-dec16-q8_0": "settings.modelsPanel.whisper.distilFrQ8",
  medium: "settings.modelsPanel.whisper.distilFr",
  small: "settings.modelsPanel.whisper.distilFrV02",
  "qwen3.5-0.8b": "settings.modelsPanel.llm.qwen35_08",
  "qwen3.5-2b": "settings.modelsPanel.llm.qwen35_2",
  "qwen3.5-4b": "settings.modelsPanel.llm.qwen35_4",
  "qwen3-0.6b": "settings.modelsPanel.llm.qwen35_08",
  "qwen3-1.7b": "settings.modelsPanel.llm.qwen35_2",
};

export function getModelDownloadLabels(t: TFunction) {
  return {
    formatTitle(kind: ModelDownloadKind): string {
      return kind === "whisper"
        ? t("window.downloadToasts.whisper")
        : t("window.downloadToasts.llm");
    },
    formatLabel(modelId: string): string {
      const key = MODEL_LABEL_KEYS[modelId];
      return key ? t(key) : modelId;
    },
  };
}
