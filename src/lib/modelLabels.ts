import type { ModelDownloadKind } from "../hooks/useModelDownloads";

const MODEL_LABELS: Record<string, string> = {
  small: "Whisper Small",
  "distil-fr-dec16": "Distil FR",
  medium: "Distil FR",
  "qwen3-0.6b": "Qwen3 0.6B",
  "qwen3-1.7b": "Qwen3 1.7B",
  "qwen3-4b": "Qwen3 4B",
};

export function formatModelDownloadTitle(kind: ModelDownloadKind): string {
  return kind === "whisper"
    ? "Téléchargement Whisper"
    : "Téléchargement LLM";
}

export function formatModelLabel(modelId: string): string {
  return MODEL_LABELS[modelId] ?? modelId;
}
