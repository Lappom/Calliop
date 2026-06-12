import type { TFunction } from "i18next";
import type { ModelDownloadKind } from "../hooks/useModelDownloads";

const MODEL_LABEL_KEYS: Record<string, string> = {
  small: "settings.modelsPanel.whisper.small",
  "distil-fr-dec16": "settings.modelsPanel.whisper.distilFr",
  medium: "settings.modelsPanel.whisper.distilFr",
  "qwen3-0.6b": "settings.modelsPanel.llm.qwen06",
  "qwen3-1.7b": "settings.modelsPanel.llm.qwen17",
  "qwen3.5-4b": "settings.modelsPanel.llm.qwen35_4",
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
