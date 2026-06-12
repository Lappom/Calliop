import type { PipelineState } from "../../hooks/usePipelineState";
import { AUDIO_BAND_COUNT } from "../../hooks/usePipelineState";

export const BAR_COUNT = AUDIO_BAND_COUNT;
export const MIN_BAR_SCALE = 0.14;
/** Headroom above normal speech; loud voice can approach this */
export const MAX_BAR_SCALE = 0.72;

/** Edge bars reach ~32 % of center sensitivity */
const EDGE_SENSITIVITY = 0.32;

/** Soft-knee compression — responsive but avoids instant saturation */
function compressVoice(amount: number): number {
  const clamped = Math.min(1, Math.max(0, amount));
  return (1 - Math.exp(-clamped * 3.4)) * 0.9;
}

export interface BarSpring {
  position: number;
  velocity: number;
  stiffness: number;
  damping: number;
}

export interface WaveformPhysicsState {
  bars: BarSpring[];
  prevLevel: number;
  smoothedLevel: number;
}

export interface WaveformTickInput {
  bands: number[];
  level: number;
  state: PipelineState;
  timeSec: number;
  dt: number;
  reducedMotion: boolean;
}

export interface WaveformTickResult {
  scales: number[];
  opacities: number[];
  changed: boolean;
}

function createBarSprings(): BarSpring[] {
  return Array.from({ length: BAR_COUNT }, (_, i) => ({
    position: MIN_BAR_SCALE,
    velocity: 0,
    stiffness: 0.085 + (i % 5) * 0.01,
    damping: 0.83 + (i % 4) * 0.02,
  }));
}

export function createWaveformPhysicsState(): WaveformPhysicsState {
  return {
    bars: createBarSprings(),
    prevLevel: 0,
    smoothedLevel: 0,
  };
}

function normalizeBand(value: number): number {
  const boosted = Math.min(1, value * 0.8);
  return Math.pow(boosted, 1.0);
}

/** Quadratic falloff — center reacts strongly, horizontal edges stay subdued */
function centerBoost(index: number): number {
  const center = (BAR_COUNT - 1) / 2;
  const distance = Math.abs(index - center) / center;
  const t = 1 - distance * distance;
  return EDGE_SENSITIVITY + (1 - EDGE_SENSITIVITY) * t;
}

function idleBreath(index: number, timeSec: number): number {
  const phase = index * 0.63;
  const gain = centerBoost(index);
  const w1 = Math.sin(timeSec * 2.17 + phase) * 0.035 * gain;
  const w2 = Math.sin(timeSec * 3.41 + phase * 1.37) * 0.024 * gain;
  const w3 = Math.sin(timeSec * 1.53 + phase * 0.81) * 0.018 * gain;
  return MIN_BAR_SCALE + 0.025 + w1 + w2 + w3;
}

function processingWave(index: number, timeSec: number): number {
  const phase = index * 0.48;
  return (
    MIN_BAR_SCALE +
    0.1 +
    Math.sin(timeSec * 4.2 + phase) * 0.05 +
    Math.sin(timeSec * 2.6 + phase * 1.2) * 0.04
  );
}

function normalizeLevel(level: number): number {
  const boosted = Math.min(1, level * 13.5);
  return Math.pow(boosted, 0.9);
}

export function tickWaveformPhysics(
  physics: WaveformPhysicsState,
  input: WaveformTickInput,
): WaveformTickResult {
  const { bands, level, state, timeSec, dt, reducedMotion } = input;
  const isRecording = state === "recording";
  const isProcessing = state === "transcribing" || state === "injecting";
  const isIdle = state === "idle" || state === "error";

  const globalLevel = normalizeLevel(level);
  const levelDelta = globalLevel - physics.prevLevel;
  physics.prevLevel = globalLevel;

  const levelCoeff = globalLevel > physics.smoothedLevel ? 0.34 : 0.058;
  physics.smoothedLevel +=
    (globalLevel - physics.smoothedLevel) * levelCoeff * dt;

  const scales: number[] = new Array(BAR_COUNT);
  const opacities: number[] = new Array(BAR_COUNT);
  let changed = false;

  for (let i = 0; i < BAR_COUNT; i++) {
    const bar = physics.bars[i];
    let target: number;

    if (isIdle) {
      target = MIN_BAR_SCALE;
    } else if (isProcessing) {
      target = processingWave(i, timeSec);
    } else if (isRecording) {
      const bandValue = normalizeBand(bands[i] ?? 0);
      const edgeGain = centerBoost(i);
      const voiceInput =
        bandValue * edgeGain * (0.32 + physics.smoothedLevel * 0.5);
      const voiceTarget = Math.min(
        MAX_BAR_SCALE,
        Math.max(
          MIN_BAR_SCALE,
          MIN_BAR_SCALE +
            (MAX_BAR_SCALE - MIN_BAR_SCALE) * compressVoice(voiceInput),
        ),
      );

      if (physics.smoothedLevel < 0.09) {
        target = reducedMotion ? MIN_BAR_SCALE : idleBreath(i, timeSec);
      } else {
        const transient =
          levelDelta > 0.04
            ? Math.min(0.08, levelDelta * edgeGain * 0.82)
            : 0;
        target = Math.min(MAX_BAR_SCALE, voiceTarget + transient);
      }
    } else {
      target = MIN_BAR_SCALE;
    }

    const prevPosition = bar.position;

    if (reducedMotion) {
      bar.velocity = 0;
      bar.position = target;
    } else {
      bar.velocity += (target - bar.position) * bar.stiffness * dt;
      bar.velocity *= Math.pow(bar.damping, dt);
      bar.position += bar.velocity * dt;
    }

    const clamped = Math.min(
      MAX_BAR_SCALE,
      Math.max(MIN_BAR_SCALE, bar.position),
    );
    scales[i] = clamped;
    opacities[i] = isRecording
      ? 0.75 + clamped * 0.25
      : isProcessing
        ? 0.65
        : 0.35;

    if (Math.abs(clamped - prevPosition) > 0.008) {
      changed = true;
    }
  }

  return { scales, opacities, changed };
}
