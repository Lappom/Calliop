import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

export const HISTORY_PAGE_SIZE = 20;

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

interface LoadEntriesOptions {
  query?: string;
  page?: number;
}

export function useHistory() {
  const [entries, setEntries] = useState<DictationEntry[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [actionEntryId, setActionEntryId] = useState<number | null>(null);
  const [page, setPage] = useState(0);
  const [totalCount, setTotalCount] = useState(0);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [entryFeedback, setEntryFeedback] = useState<
    Record<number, "copied" | "injected">
  >({});
  const activeQueryRef = useRef("");
  const activePageRef = useRef(0);

  const loadEntries = useCallback(async (options?: LoadEntriesOptions) => {
    try {
      const trimmed = (options?.query ?? activeQueryRef.current).trim();
      const queryChanged =
        options?.query !== undefined && trimmed !== activeQueryRef.current;
      const nextPage = queryChanged
        ? 0
        : (options?.page ?? activePageRef.current);
      const offset = nextPage * HISTORY_PAGE_SIZE;

      activeQueryRef.current = trimmed;
      activePageRef.current = nextPage;
      setPage(nextPage);

      const [result, total] = await Promise.all([
        trimmed.length > 0
          ? invoke<DictationEntry[]>("search_dictations", {
              query: trimmed,
              limit: HISTORY_PAGE_SIZE,
              offset,
            })
          : invoke<DictationEntry[]>("list_dictations", {
              limit: HISTORY_PAGE_SIZE,
              offset,
            }),
        trimmed.length > 0
          ? invoke<number>("count_search_dictations", { query: trimmed })
          : invoke<number>("count_dictations"),
      ]);

      setEntries(result);
      setTotalCount(total);
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
    setEntryFeedback({ [id]: "copied" });
    const resetTimer = window.setTimeout(() => {
      setEntryFeedback((prev) => (prev[id] === "copied" ? {} : prev));
    }, 1500);

    setActionEntryId(id);
    try {
      await invoke("copy_dictation", { id });
      setErrorMessage(null);
    } catch (err) {
      window.clearTimeout(resetTimer);
      setEntryFeedback((prev) => (prev[id] === "copied" ? {} : prev));
      setErrorMessage(String(err));
    } finally {
      setActionEntryId(null);
    }
  }, []);

  const reinjectEntry = useCallback(async (id: number) => {
    setActionEntryId(id);
    try {
      await invoke("reinject_dictation", { id });
      setEntryFeedback({ [id]: "injected" });
      setErrorMessage(null);
    } catch (err) {
      setErrorMessage(String(err));
    } finally {
      setActionEntryId(null);
    }
  }, []);

  const goToPage = useCallback(
    (nextPage: number) => {
      void loadEntries({ page: nextPage });
    },
    [loadEntries],
  );

  return {
    entries,
    loaded,
    actionEntryId,
    page,
    totalCount,
    pageSize: HISTORY_PAGE_SIZE,
    errorMessage,
    entryFeedback,
    loadEntries,
    goToPage,
    copyEntry,
    reinjectEntry,
  };
}
