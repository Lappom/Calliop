import { useEffect, useRef, useState } from "react";
import type { PipelineState } from "../../hooks/usePipelineState";

interface WaveformVisualizerProps {
  state: PipelineState;
  level: number;
}

const BAR_COUNT = 14;
const MAX_BAR_HEIGHT = 22;
const ATTACK = 0.58;
const DECAY = 0.08;

function normalizeLevel(level: number): number {
  const boosted = Math.min(1, level * 24);
  return Math.pow(boosted, 0.62);
}

export function WaveformVisualizer({ state, level }: WaveformVisualizerProps) {
  const isRecording = state === "recording";
  const isProcessing = state === "transcribing" || state === "injecting";

  const targetRef = useRef(0);
  const smoothedRef = useRef(0);
  const timeRef = useRef(0);
  const [bars, setBars] = useState<number[]>(() =>
    Array.from({ length: BAR_COUNT }, () => 0.14),
  );

  targetRef.current = normalizeLevel(level);

  useEffect(() => {
    let raf = 0;

    const tick = (now: number) => {
      const prevTime = timeRef.current;
      timeRef.current = now;
      const dt = prevTime > 0 ? Math.min((now - prevTime) / 16, 2) : 1;

      const target = targetRef.current;
      const prev = smoothedRef.current;
      const coeff = target > prev ? ATTACK : DECAY;
      const smoothed = prev + (target - prev) * coeff * dt;
      smoothedRef.current = smoothed;

      const t = now / 1000;
      const nextBars = Array.from({ length: BAR_COUNT }, (_, i) => {
        if (isProcessing) {
          return 0.14 + (i % 3) * 0.06;
        }

        const center = (BAR_COUNT - 1) / 2;
        const centerDistance = Math.abs(i - center) / center;
        const centerBoost = 1 - centerDistance * 0.18;
        const barOffset = ((i % 5) + 1) * 0.1;
        const idlePulse =
          smoothed < 0.08
            ? 0.18 + Math.sin(t * 3.6 + i * 0.5) * 0.1
            : 0;
        const active = smoothed * centerBoost * (0.88 + barOffset);
        return Math.min(1, Math.max(0.14, active + idlePulse));
      });

      setBars(nextBars);
      raf = requestAnimationFrame(tick);
    };

    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [isProcessing]);

  return (
    <div
      className="flex h-[22px] items-center justify-center gap-[2px]"
      aria-hidden="true"
    >
      {bars.map((height, i) => (
        <span
          key={i}
          className={[
            "w-[2.5px] shrink-0 rounded-full transition-[height,background-color] duration-75",
            isProcessing
              ? "bg-accent-blue/60"
              : isRecording
                ? "bg-accent-green"
                : "bg-ink/30",
          ].join(" ")}
          style={{
            height: `${Math.max(4, Math.round(height * MAX_BAR_HEIGHT))}px`,
          }}
        />
      ))}
    </div>
  );
}
