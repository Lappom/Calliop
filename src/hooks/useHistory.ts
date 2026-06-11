import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

export interface DictationEntry {
  id: number;
  text: string;
  wordCount: number;
  audioDurationMs: number;
  sttMs: number;
  llmMs: number;
  injectMs: number;
  totalMs: number;
  appExe: string | null;
  appTitle: string | null;
  created_at: string;
}

export function useHistory() {
  const [entries, setEntries] = useState<DictationEntry[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [entryFeedback, setEntryFeedback] = useState<
    Record<number, "copied" | "injected">
  >({});

  const loadEntries = useCallback(async (query?: string) => {
    try {
      const trimmed = query?.trim() ?? "";
      const result =
        trimmed.length > 0
          ? await invoke<DictationEntry[]>("search_dictations", {
              query: trimmed,
            })
          : await invoke<DictationEntry[]>("list_dictations", {});
      setEntries(result);
      setLoaded(true);
      setErrorMessage(null);
    } catch (err) {
      setErrorMessage(String(err));
    }
  }, []);

  useEffect(() => {
    let cancelled = false;

    const setup = async () => {
      if (!cancelled) {
        await loadEntries();
      }
    };

    void setup();

    const unlisten = listen("history-updated", () => {
      void loadEntries();
    });

    return () => {
      cancelled = true;
      void unlisten.then((drop) => drop());
    };
  }, [loadEntries]);

  const copyEntry = useCallback(async (id: number) => {
    setBusy(true);
    try {
      await invoke("copy_dictation", { id });
      setEntryFeedback({ [id]: "copied" });
      setErrorMessage(null);
    } catch (err) {
      setErrorMessage(String(err));
    } finally {
      setBusy(false);
    }
  }, []);

  const reinjectEntry = useCallback(async (id: number) => {
    setBusy(true);
    try {
      await invoke("reinject_dictation", { id });
      setEntryFeedback({ [id]: "injected" });
      setErrorMessage(null);
    } catch (err) {
      setErrorMessage(String(err));
    } finally {
      setBusy(false);
    }
  }, []);

  return {
    entries,
    loaded,
    busy,
    errorMessage,
    entryFeedback,
    loadEntries,
    copyEntry,
    reinjectEntry,
  };
}
