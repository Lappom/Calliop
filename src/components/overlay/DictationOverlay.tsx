import { useSttLanguage } from "../../hooks/useSttLanguage";
import {
  pipelineGlow,
  pipelineStatusColor,
  STATE_LABELS,
  usePipelineState,
} from "../../hooks/usePipelineState";
import { SectionGlow } from "../layout/SectionGlow";
import { StatusDot } from "../ui/StatusDot";
import { WaveformVisualizer } from "./WaveformVisualizer";

export function DictationOverlay() {
  const {
    pipelineState,
    partialTranscript,
    errorMessage,
    audioLevel,
  } = usePipelineState();
  const { languageLabel, cycling, cycleLanguage } = useSttLanguage();
  const glow = pipelineGlow(pipelineState, Boolean(errorMessage));
  const statusColor = pipelineStatusColor(
    pipelineState,
    Boolean(errorMessage),
  );

  return (
    <div className="flex min-h-screen items-center justify-center bg-transparent p-3">
      <SectionGlow glow={glow} className="w-full max-w-[280px]">
        <div
          className={[
            "flex flex-col gap-2 rounded-lg border border-hairline-strong",
            "bg-surface-card/95 px-4 py-3 backdrop-blur-sm",
          ].join(" ")}
          role="status"
          aria-live="polite"
        >
          <div className="flex items-center gap-3">
            <StatusDot color={statusColor} />
            <div className="min-w-0 flex-1">
              <p className="truncate text-body-sm font-medium text-ink">
                {STATE_LABELS[pipelineState]}
              </p>
              <WaveformVisualizer state={pipelineState} level={audioLevel} />
            </div>
            {pipelineState === "recording" && (
              <button
                type="button"
                disabled={cycling}
                onClick={() => {
                  void cycleLanguage();
                }}
                className={[
                  "shrink-0 rounded border border-hairline-strong px-2 py-0.5",
                  "font-[family-name:var(--font-body)] text-[10px] font-semibold tracking-wider text-charcoal",
                  "cursor-pointer transition-colors hover:bg-surface-elevated hover:text-ink",
                  "disabled:cursor-not-allowed disabled:opacity-50",
                ].join(" ")}
                aria-label={`Langue de dictée : ${languageLabel}. Cliquer pour changer.`}
              >
                {languageLabel}
              </button>
            )}
          </div>
          {partialTranscript && (
            <p
              className="truncate text-caption text-charcoal"
              aria-hidden="true"
            >
              {partialTranscript}
            </p>
          )}
        </div>
      </SectionGlow>
    </div>
  );
}
