import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

export interface LatencySnapshot {
  sttMs: number;
  llmMs: number;
  injectMs: number;
  totalMs: number;
}

export interface AppUsageEntry {
  exeName: string;
  dictationCount: number;
  wordCount: number;
}

export interface DailyActivityEntry {
  date: string;
  wordCount: number;
  dictationCount: number;
}

export interface RecentLatencyEntry {
  sttMs: number;
  llmMs: number;
  injectMs: number;
  totalMs: number;
  created_at: string;
}

export interface Insights {
  lastLatency: LatencySnapshot | null;
  wordsToday: number;
  totalWords: number;
  averageWpm: number;
  wpmVsTypingPercent: number;
  learnedCorrections: number;
  appUsage: AppUsageEntry[];
  dailyActivity: DailyActivityEntry[];
  recentLatency: RecentLatencyEntry[];
}

export function useInsights() {
  const [insights, setInsights] = useState<Insights | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const loadInsights = useCallback(async () => {
    try {
      const result = await invoke<Insights>("get_insights");
      setInsights(result);
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
        await loadInsights();
      }
    };

    void setup();

    const unlistenHistory = listen("history-updated", () => {
      void loadInsights();
    });
    const unlistenDictionary = listen("dictionary-updated", () => {
      void loadInsights();
    });

    return () => {
      cancelled = true;
      void unlistenHistory.then((drop) => drop());
      void unlistenDictionary.then((drop) => drop());
    };
  }, [loadInsights]);

  return { insights, loaded, errorMessage, reload: loadInsights };
}
