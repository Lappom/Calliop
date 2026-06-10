import type { PipelineState } from "../../hooks/usePipelineState";

interface WaveformVisualizerProps {
  state: PipelineState;
  level: number;
}

const BAR_COUNT = 12;

export function WaveformVisualizer({ state, level }: WaveformVisualizerProps) {
  const isRecording = state === "recording";
  const isActive = state === "transcribing" || state === "injecting";
  const normalized = Math.min(1, level * 8);

  return (
    <div
      className="flex h-6 items-end justify-center gap-0.5"
      aria-hidden="true"
    >
      {Array.from({ length: BAR_COUNT }, (_, i) => {
        const wave = isRecording
          ? 0.25 + normalized * (0.5 + ((i % 4) + 1) * 0.12)
          : isActive
            ? 0.35 + (i % 3) * 0.12
            : 0.08;

        return (
          <span
            key={i}
            className={[
              "w-1 rounded-full bg-ink/60 transition-[height] duration-75",
              isRecording && "animate-waveform",
              isActive && "animate-pulse",
            ]
              .filter(Boolean)
              .join(" ")}
            style={{
              height: `${Math.round(Math.min(1, wave) * 100)}%`,
              animationDelay: isRecording ? `${i * 0.08}s` : undefined,
            }}
          />
        );
      })}
    </div>
  );
}
