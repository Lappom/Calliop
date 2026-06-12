import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export type ModelDownloadKind = "whisper" | "llm";

export interface ActiveModelDownload {
  kind: ModelDownloadKind;
  percent: number;
  modelId: string;
}

interface DownloadProgressPayload {
  model_id: string;
  downloaded: number;
  total: number | null;
  percent: number;
  source: string;
}

export function useModelDownloads(): ActiveModelDownload[] {
  const [downloads, setDownloads] = useState<ActiveModelDownload[]>([]);

  useEffect(() => {
    const upsertDownload = (
      kind: ModelDownloadKind,
      modelId: string,
      percent: number,
    ) => {
      setDownloads((current) => {
        const index = current.findIndex((entry) => entry.kind === kind);
        const next = { kind, modelId, percent };
        if (index === -1) {
          return [...current, next];
        }
        const updated = [...current];
        updated[index] = next;
        return updated;
      });
    };

    const clearDownload = (kind: ModelDownloadKind) => {
      setDownloads((current) => current.filter((entry) => entry.kind !== kind));
    };

    const unlisteners = Promise.all([
      listen<DownloadProgressPayload>("model-download-progress", (event) => {
        upsertDownload(
          "whisper",
          event.payload.model_id,
          event.payload.percent,
        );
      }),
      listen("model-ready", () => {
        clearDownload("whisper");
      }),
      listen<DownloadProgressPayload>("llm-model-download-progress", (event) => {
        upsertDownload("llm", event.payload.model_id, event.payload.percent);
      }),
      listen("llm-ready", () => {
        clearDownload("llm");
      }),
    ]);

    return () => {
      void unlisteners.then((drops) => drops.forEach((drop) => drop()));
    };
  }, []);

  return downloads;
}
