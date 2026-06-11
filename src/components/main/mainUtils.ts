import type { TFunction } from "i18next";
import type { LatencyMetricsPayload, PipelineState } from "../../hooks/usePipelineState";
import type { GlowColor, GlowPulse } from "../layout/glowSurface";

export function getStateHints(t: TFunction): Record<PipelineState, string> {
  return {
    idle: t("main.pipeline.states.idle.hint"),
    recording: t("main.pipeline.states.recording.hint"),
    transcribing: t("main.pipeline.states.transcribing.hint"),
    injecting: t("main.pipeline.states.injecting.hint"),
    error: t("main.pipeline.states.error.hint"),
  };
}

export function pipelineStateLabel(t: TFunction, state: PipelineState): string {
  return t(`main.pipeline.states.${state}.label`);
}

export function formatHotkeyDisplay(hotkey: string, t: TFunction): string {
  return hotkey.replace(/Space/g, t("keys.space"));
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

export function pipelineCardGlowPulse(
  state: PipelineState,
  hasError: boolean,
  isDownloading: boolean,
): GlowPulse | undefined {
  if (isDownloading) return "normal";
  if (hasError || state === "error") return "slow";
  if (state === "recording") return "normal";
  if (state === "transcribing" || state === "injecting") return "slow";
  return undefined;
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
  if (metrics.llmStatus === "skipped" || metrics.llmStatus === "failed") {
    llm = metrics.llmSkipReason ?? metrics.llmStatus;
  } else if (metrics.llmBlockedMs != null && metrics.llmBlockedMs > 0) {
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
