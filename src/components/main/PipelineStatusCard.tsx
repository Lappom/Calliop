import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { PipelineState } from "../../hooks/usePipelineState";
import { pipelineStatusColor } from "../../hooks/usePipelineState";
import { translateError } from "../../lib/translateError";
import { glowSurfaceClasses } from "../layout/glowSurface";
import { ProgressBar } from "../ui/ProgressBar";
import { StatusDot } from "../ui/StatusDot";
import { AudioLevelBars } from "./AudioLevelBars";
import {
  getStateHints,
  isPipelineBusy,
  pipelineCardGlow,
  pipelineStateLabel,
} from "./mainUtils";

interface PipelineStatusCardProps {
  pipelineState: PipelineState;
  errorMessage: string | null;
  modelReady: boolean;
  modelProgress: number | null;
  partialTranscript: string;
  audioLevel: number;
}

export function PipelineStatusCard({
  pipelineState,
  errorMessage,
  modelReady,
  modelProgress,
  partialTranscript,
  audioLevel,
}: PipelineStatusCardProps) {
  const { t } = useTranslation();
  const stateHints = useMemo(() => getStateHints(t), [t]);
  const hasError = Boolean(errorMessage) || pipelineState === "error";
  const glow = pipelineCardGlow(pipelineState, hasError);
  const statusColor = pipelineStatusColor(pipelineState, hasError);
  const busy = isPipelineBusy(pipelineState);
  const showPartial = partialTranscript.length > 0 && pipelineState === "recording";

  if (!modelReady && modelProgress !== null) {
    return (
      <div
        className={[
          glowSurfaceClasses("orange"),
          "rounded-lg border border-hairline-strong bg-surface-card p-5 sm:p-6",
        ].join(" ")}
      >
        <p className="text-caption relative m-0 text-charcoal">
          {t("main.pipeline.preparing")}
        </p>
        <p className="text-heading-sm relative m-0 mt-2 text-ink">
          {t("main.pipeline.downloadWhisper")}
        </p>
        <div className="relative mt-4">
          <ProgressBar value={modelProgress} />
        </div>
      </div>
    );
  }

  const displayError = errorMessage
    ? translateError(errorMessage, t)
    : t("main.pipeline.states.error.unknown");

  return (
    <div
      className={[
        glow ? glowSurfaceClasses(glow) : "",
        "rounded-lg border border-hairline-strong bg-surface-card p-5 sm:p-6",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className="relative flex flex-wrap items-start justify-between gap-4">
        <div className="flex min-w-0 items-start gap-3">
          <StatusDot
            color={statusColor}
            className={busy ? "mt-1.5 animate-pulse" : "mt-1.5"}
          />
          <div className="min-w-0">
            <p className="text-heading-sm m-0 text-ink">
              {hasError
                ? pipelineStateLabel(t, "error")
                : pipelineStateLabel(t, pipelineState)}
            </p>
            <p className="text-body-sm mt-1.5 text-charcoal">
              {hasError ? displayError : stateHints[pipelineState]}
            </p>
          </div>
        </div>
        {pipelineState === "recording" && (
          <AudioLevelBars level={audioLevel} />
        )}
      </div>

      {showPartial && (
        <div className="relative mt-5 overflow-hidden rounded-md border border-hairline bg-surface-deep px-4 py-3">
          <p className="text-caption m-0 text-ash">
            {t("main.pipeline.liveTranscript")}
          </p>
          <p className="text-body-md m-0 mt-2 text-body">{partialTranscript}</p>
        </div>
      )}
    </div>
  );
}
