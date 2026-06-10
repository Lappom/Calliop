import {
  pipelineGlow,
  pipelineStatusColor,
  STATE_LABELS,
  usePipelineState,
} from "../../hooks/usePipelineState";
import { SectionGlow } from "../layout/SectionGlow";
import { StatusDot } from "../ui/StatusDot";
import { WaveformStub } from "./WaveformStub";

export function DictationOverlay() {
  const { pipelineState, errorMessage } = usePipelineState();
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
            "flex items-center gap-3 rounded-lg border border-hairline-strong",
            "bg-surface-card/95 px-4 py-3 backdrop-blur-sm",
          ].join(" ")}
          role="status"
          aria-live="polite"
        >
          <StatusDot color={statusColor} />
          <div className="min-w-0 flex-1">
            <p className="truncate text-body-sm font-medium text-ink">
              {STATE_LABELS[pipelineState]}
            </p>
            <WaveformStub state={pipelineState} />
          </div>
        </div>
      </SectionGlow>
    </div>
  );
}
