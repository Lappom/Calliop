import { useTranslation } from "react-i18next";
import { usePipelineState, type PipelineState } from "../../hooks/usePipelineState";
import { WaveformVisualizer } from "./WaveformVisualizer";

const OVERLAY_STATE_KEYS: Record<PipelineState, string> = {
  idle: "overlay.states.idle",
  recording: "overlay.states.recording",
  transcribing: "overlay.states.transcribing",
  injecting: "overlay.states.injecting",
  error: "overlay.states.error",
};

function glowStateClass(state: PipelineState, hasError: boolean): string {
  if (hasError || state === "error") {
    return "overlay-glow-red";
  }
  if (state === "recording") {
    return "overlay-glow-green overlay-breathe";
  }
  if (state === "transcribing" || state === "injecting") {
    return "overlay-glow-blue overlay-shimmer";
  }
  return "";
}

export function DictationOverlay() {
  const { t } = useTranslation();
  const { pipelineState, errorMessage, audioLevel, audioBands } =
    usePipelineState();
  const hasError = Boolean(errorMessage);

  return (
    <div
      className={[
        "overlay-glow",
        "animate-overlay-pill-in",
        glowStateClass(pipelineState, hasError),
      ]
        .filter(Boolean)
        .join(" ")}
      role="status"
      aria-live="polite"
      aria-label={t(OVERLAY_STATE_KEYS[pipelineState])}
    >
      <div className="overlay-pill">
        <WaveformVisualizer
          state={pipelineState}
          level={audioLevel}
          bands={audioBands}
        />
      </div>
    </div>
  );
}
