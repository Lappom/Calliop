import { usePipelineState, type PipelineState } from "../../hooks/usePipelineState";
import { WaveformVisualizer } from "./WaveformVisualizer";

const ARIA_LABELS: Record<PipelineState, string> = {
  idle: "En attente",
  recording: "Écoute en cours",
  transcribing: "Transcription en cours",
  injecting: "Injection du texte",
  error: "Erreur de dictée",
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
  const { pipelineState, errorMessage, audioLevel } = usePipelineState();
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
      aria-label={ARIA_LABELS[pipelineState]}
    >
      <div className="overlay-pill">
        <WaveformVisualizer state={pipelineState} level={audioLevel} />
      </div>
    </div>
  );
}
