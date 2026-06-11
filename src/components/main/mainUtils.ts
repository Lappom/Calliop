import type { LatencyMetricsPayload, PipelineState } from "../../hooks/usePipelineState";
import type { GlowColor } from "../layout/glowSurface";

export const STATE_HINTS: Record<PipelineState, string> = {
  idle: "Placez le curseur dans votre application, puis lancez une dictée.",
  recording: "Parlez clairement — réappuyez pour arrêter ou relâchez en push-to-talk.",
  transcribing: "Whisper transcrit votre voix localement, sans envoi réseau.",
  injecting: "Le texte est inséré dans l'application active.",
  error: "Consultez le message ci-dessous ou relancez une dictée.",
};

export function formatHotkeyDisplay(hotkey: string): string {
  return hotkey.replace(/Space/g, "Espace");
}

export function hotkeyParts(hotkey: string): string[] {
  return hotkey.split("+").map((part) => part.trim());
}

export function pipelineCardGlow(
  state: PipelineState,
  hasError: boolean,
): GlowColor | null {
  if (hasError || state === "error") return "red";
  if (state === "recording") return "green";
  if (state === "transcribing" || state === "injecting") return "blue";
  return null;
}

export function isPipelineBusy(state: PipelineState): boolean {
  return state === "recording" || state === "transcribing" || state === "injecting";
}

export function formatLatencyBreakdown(metrics: LatencyMetricsPayload): {
  stt: string;
  llm: string | null;
  inject: string;
  total: string;
} {
  const stt =
    metrics.sttWaitMs != null
      ? `${metrics.sttWaitMs + metrics.sttMs} ms`
      : `${metrics.sttMs} ms`;

  let llm: string | null = null;
  if (metrics.llmBlockedMs != null && metrics.llmBlockedMs > 0) {
    llm = `${metrics.llmBlockedMs} ms`;
  } else if (metrics.llmMs > 0) {
    llm = `${metrics.llmMs} ms`;
  }

  return {
    stt,
    llm,
    inject: `${metrics.injectMs} ms`,
    total: `${metrics.totalMs} ms`,
  };
}
