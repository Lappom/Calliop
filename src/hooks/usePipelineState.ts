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

export interface LatencyMetricsPayload {
  sttMs: number;
  injectMs: number;
  totalMs: number;
}

export const STATE_LABELS: Record<PipelineState, string> = {
  idle: "En attente",
  recording: "Écoute en cours…",
  transcribing: "Transcription…",
  injecting: "Injection du texte…",
  error: "Erreur",
};

export function usePipelineState() {
  const [pipelineState, setPipelineState] = useState<PipelineState>("idle");
  const [lastTranscript, setLastTranscript] = useState<string | null>(null);
  const [partialTranscript, setPartialTranscript] = useState("");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [modelReady, setModelReady] = useState(false);
  const [modelProgress, setModelProgress] = useState<number | null>(null);
  const [audioLevel, setAudioLevel] = useState(0);
  const [latencyMetrics, setLatencyMetrics] =
    useState<LatencyMetricsPayload | null>(null);

  useEffect(() => {
    let cancelled = false;

    const setup = async () => {
      try {
        const state = await invoke<string>("get_pipeline_state");
        if (!cancelled) {
          setPipelineState(state as PipelineState);
        }
      } catch {
        // Backend not ready yet.
      }

      try {
        await invoke("ensure_model");
        if (!cancelled) {
          setModelReady(true);
          setModelProgress(null);
        }
      } catch (err) {
        if (!cancelled) {
          setErrorMessage(String(err));
        }
      }
    };

    void setup();

    const unlisteners = Promise.all([
      listen<PipelineStatePayload>("pipeline-state", (event) => {
        setPipelineState(event.payload.state);
        if (event.payload.state === "error") {
          setErrorMessage(event.payload.message ?? "Erreur inconnue");
        } else {
          setErrorMessage(null);
        }
        if (event.payload.message && event.payload.state === "idle") {
          setLastTranscript(event.payload.message);
          setPartialTranscript("");
        }
      }),
      listen("model-ready", () => {
        setModelReady(true);
        setModelProgress(null);
      }),
      listen<ModelDownloadProgress>("model-download-progress", (event) => {
        setModelProgress(event.payload.percent);
      }),
      listen<PartialTranscriptPayload>("partial-transcript", (event) => {
        setPartialTranscript((current) => {
          const next = current
            ? `${current} ${event.payload.text}`
            : event.payload.text;
          return next.trim();
        });
      }),
      listen("partial-transcript-reset", () => {
        setPartialTranscript("");
      }),
      listen<{ level: number }>("audio-level", (event) => {
        setAudioLevel(event.payload.level);
      }),
      listen<LatencyMetricsPayload>("latency-metrics", (event) => {
        setLatencyMetrics(event.payload);
      }),
    ]);

    return () => {
      cancelled = true;
      void unlisteners.then((drops) => drops.forEach((drop) => drop()));
    };
  }, []);

  return {
    pipelineState,
    lastTranscript,
    partialTranscript,
    errorMessage,
    modelReady,
    modelProgress,
    audioLevel,
    latencyMetrics,
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
