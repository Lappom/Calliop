import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export type PipelineState =
  | "idle"
  | "recording"
  | "transcribing"
  | "injecting"
  | "error";

interface PipelineStatePayload {
  state: PipelineState;
  message?: string | null;
}

interface ModelDownloadProgress {
  downloaded: number;
  total: number | null;
  percent: number;
  source: string;
}

export interface PartialTranscriptPayload {
  text: string;
  segmentIndex: number;
}

export type LlmStatus = "applied" | "skipped" | "failed" | "disabled";

export const AUDIO_BAND_COUNT = 14;

export interface AudioLevelPayload {
  level: number;
  bands?: number[];
}

export interface LatencyMetricsPayload {
  sttMs: number;
  sttWaitMs?: number;
  llmMs: number;
  llmBlockedMs?: number;
  injectMs: number;
  totalMs: number;
  llmStatus?: LlmStatus;
  llmSkipReason?: string | null;
  recordStartMs?: number;
  micOpenMs?: number;
}

export interface RecordStartMetricsPayload {
  recordStartMs: number;
  micOpenMs: number;
}

export function usePipelineState() {
  const [pipelineState, setPipelineState] = useState<PipelineState>("idle");
  const [lastTranscript, setLastTranscript] = useState<string | null>(null);
  const [transcriptRevision, setTranscriptRevision] = useState(0);
  const [partialTranscript, setPartialTranscript] = useState("");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [modelReady, setModelReady] = useState(false);
  const [modelProgress, setModelProgress] = useState<number | null>(null);
  const [audioLevel, setAudioLevel] = useState(0);
  const [audioBands, setAudioBands] = useState<number[]>(() =>
    Array.from({ length: AUDIO_BAND_COUNT }, () => 0),
  );
  const [latencyMetrics, setLatencyMetrics] =
    useState<LatencyMetricsPayload | null>(null);
  const [busyHint, setBusyHint] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const unlisteners = Promise.all([
      listen<PipelineStatePayload>("pipeline-state", (event) => {
        setPipelineState(event.payload.state);
        if (event.payload.state === "idle") {
          setBusyHint(null);
        }
        if (event.payload.state === "error") {
          setErrorMessage(event.payload.message ?? null);
          setPartialTranscript("");
        } else {
          setErrorMessage(null);
        }
        if (event.payload.message && event.payload.state === "idle") {
          setLastTranscript(event.payload.message);
          setTranscriptRevision((revision) => revision + 1);
          setPartialTranscript("");
        }
      }),
      listen("model-ready", () => {
        setModelReady(true);
        setModelProgress(null);
      }),
      listen("model-unready", () => {
        setModelReady(false);
        setModelProgress(null);
      }),
      listen<ModelDownloadProgress>("model-download-progress", (event) => {
        setModelProgress((current) => {
          const next = event.payload.percent;
          return current === null || next >= current ? next : current;
        });
      }),
      listen<string>("model-init-error", (event) => {
        setErrorMessage(event.payload);
      }),
      listen<PartialTranscriptPayload>("partial-transcript", (event) => {
        setPartialTranscript(event.payload.text.trim());
      }),
      listen("partial-transcript-reset", () => {
        setPartialTranscript("");
      }),
      listen<AudioLevelPayload>("audio-level", (event) => {
        const { level, bands } = event.payload;
        setAudioLevel(level);
        if (bands && bands.length > 0) {
          setAudioBands(bands.slice(0, AUDIO_BAND_COUNT));
        } else {
          const uniform = Math.min(1, level * 24);
          setAudioBands(
            Array.from({ length: AUDIO_BAND_COUNT }, () => uniform),
          );
        }
      }),
      listen<LatencyMetricsPayload>("latency-metrics", (event) => {
        setLatencyMetrics(event.payload);
      }),
      listen<{ state: string; cancelable: boolean }>("dictation-busy", (event) => {
        setBusyHint(
          event.payload.cancelable
            ? "main.pipeline.states.busyProcessing"
            : "main.pipeline.states.busyInjecting",
        );
      }),
      listen("dictation-cancelled", () => {
        setBusyHint(null);
      }),
    ]);

    const setup = async () => {
      await unlisteners;

      try {
        const [state, ready] = await Promise.all([
          invoke<string>("get_pipeline_state"),
          invoke<boolean>("is_model_ready"),
        ]);
        if (!cancelled) {
          setPipelineState(state as PipelineState);
          if (ready) {
            setModelReady(true);
            setModelProgress(null);
          }
        }
      } catch {
        // Backend not ready yet.
      }
    };

    void setup();

    return () => {
      cancelled = true;
      void unlisteners.then((drops) => drops.forEach((drop) => drop()));
    };
  }, []);

  return {
    pipelineState,
    lastTranscript,
    transcriptRevision,
    partialTranscript,
    errorMessage,
    modelReady,
    modelProgress,
    audioLevel,
    audioBands,
    latencyMetrics,
    busyHint,
  };
}

export function pipelineGlow(
  state: PipelineState,
  hasError: boolean,
): "green" | "blue" | "red" | "orange" | "none" {
  if (hasError || state === "error") return "red";
  if (state === "recording") return "green";
  if (state === "transcribing" || state === "injecting") return "blue";
  return "none";
}

export function pipelineStatusColor(
  state: PipelineState,
  hasError: boolean,
): "green" | "blue" | "red" | "yellow" | "mute" {
  if (hasError || state === "error") return "red";
  if (state === "recording") return "green";
  if (state === "transcribing" || state === "injecting") return "blue";
  if (state === "idle") return "mute";
  return "yellow";
}
