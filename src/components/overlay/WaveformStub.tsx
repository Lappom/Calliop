import type { PipelineState } from "../../hooks/usePipelineState";

interface WaveformStubProps {
  state: PipelineState;
}

export function WaveformStub({ state }: WaveformStubProps) {
  const isRecording = state === "recording";
  const isActive = state === "transcribing" || state === "injecting";

  return (
    <div
      className="flex h-6 items-end justify-center gap-0.5"
      aria-hidden="true"
    >
      {Array.from({ length: 12 }, (_, i) => (
        <span
          key={i}
          className={[
            "w-1 rounded-full bg-ink/60",
            isRecording && "animate-waveform",
            isActive && "animate-pulse",
            !isRecording && !isActive && "h-1",
          ]
            .filter(Boolean)
            .join(" ")}
          style={
            isRecording
              ? {
                  animationDelay: `${i * 0.08}s`,
                  height: `${30 + (i % 4) * 18}%`,
                }
              : isActive
                ? { height: `${40 + (i % 3) * 15}%` }
                : undefined
          }
        />
      ))}
    </div>
  );
}
