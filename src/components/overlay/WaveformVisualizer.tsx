import { useEffect, useRef, useState } from "react";
import type { PipelineState } from "../../hooks/usePipelineState";
import {
  BAR_COUNT,
  createWaveformPhysicsState,
  MIN_BAR_SCALE,
  tickWaveformPhysics,
} from "./waveformPhysics";

interface WaveformVisualizerProps {
  state: PipelineState;
  level: number;
  bands: number[];
}

const MAX_BAR_HEIGHT = 22;
const EPSILON_PX = 0.5;

function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

export function WaveformVisualizer({
  state,
  level,
  bands,
}: WaveformVisualizerProps) {
  const isRecording = state === "recording";
  const isProcessing = state === "transcribing" || state === "injecting";

  const levelRef = useRef(0);
  const bandsRef = useRef<number[]>(bands);
  const stateRef = useRef(state);
  const timeRef = useRef(0);
  const physicsRef = useRef(createWaveformPhysicsState());
  const reducedMotionRef = useRef(prefersReducedMotion());
  const [renderBars, setRenderBars] = useState(() =>
    Array.from({ length: BAR_COUNT }, () => MIN_BAR_SCALE),
  );
  const [renderOpacities, setRenderOpacities] = useState(() =>
    Array.from({ length: BAR_COUNT }, () => 0.35),
  );
  const lastRenderScalesRef = useRef(renderBars);

  levelRef.current = level;
  bandsRef.current = bands;
  stateRef.current = state;

  useEffect(() => {
    const media = window.matchMedia("(prefers-reduced-motion: reduce)");
    const onChange = () => {
      reducedMotionRef.current = media.matches;
    };
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, []);

  useEffect(() => {
    let raf = 0;

    const cancel = () => {
      cancelAnimationFrame(raf);
      raf = 0;
    };

    const tick = (now: number) => {
      if (document.hidden) {
        raf = 0;
        return;
      }

      const prevTime = timeRef.current;
      timeRef.current = now;
      const dt = prevTime > 0 ? Math.min((now - prevTime) / 16, 2) : 1;

      const result = tickWaveformPhysics(physicsRef.current, {
        bands: bandsRef.current,
        level: levelRef.current,
        state: stateRef.current,
        timeSec: now / 1000,
        dt,
        reducedMotion: reducedMotionRef.current,
      });

      if (result.changed) {
        const minPx = MIN_BAR_SCALE * MAX_BAR_HEIGHT;
        let meaningfulChange = false;
        for (let i = 0; i < BAR_COUNT; i++) {
          const nextPx = Math.max(minPx, result.scales[i] * MAX_BAR_HEIGHT);
          const prevPx = Math.max(
            minPx,
            lastRenderScalesRef.current[i] * MAX_BAR_HEIGHT,
          );
          if (Math.abs(nextPx - prevPx) > EPSILON_PX) {
            meaningfulChange = true;
            break;
          }
        }
        if (meaningfulChange) {
          lastRenderScalesRef.current = result.scales;
          setRenderBars(result.scales);
          setRenderOpacities(result.opacities);
        }
      }

      raf = requestAnimationFrame(tick);
    };

    const startLoop = () => {
      cancel();
      if (!document.hidden) {
        raf = requestAnimationFrame(tick);
      }
    };

    const onVisibilityChange = () => {
      if (document.hidden) {
        cancel();
      } else {
        startLoop();
      }
    };

    document.addEventListener("visibilitychange", onVisibilityChange);
    startLoop();

    return () => {
      document.removeEventListener("visibilitychange", onVisibilityChange);
      cancel();
    };
  }, []);

  return (
    <div
      className="flex h-[22px] items-center justify-center gap-[2px]"
      aria-hidden="true"
    >
      {renderBars.map((scale, i) => (
        <span
          key={i}
          className={[
            "h-[22px] w-[2.5px] shrink-0 origin-center rounded-full",
            isProcessing
              ? "bg-accent-blue/60"
              : isRecording
                ? "bg-accent-green"
                : "bg-ink/30",
          ].join(" ")}
          style={{
            transform: `scaleY(${scale})`,
            opacity: renderOpacities[i],
          }}
        />
      ))}
    </div>
  );
}
