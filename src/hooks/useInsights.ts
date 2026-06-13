import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import type { LatencyMetricsPayload } from "./usePipelineState";

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

export interface StreakInfo {
  currentStreak: number;
  bestStreak: number;
  activeToday: boolean;
}

export interface EstimatedTimeSaved {
  minutesSaved: number;
  baselineWpm: number;
}

export interface HourAppHeatmapCell {
  hour: number;
  exeName: string;
  wordCount: number;
  dictationCount: number;
}

export interface Insights {
  lastLatency: LatencySnapshot | null;
  wordsToday: number;
  dictationsToday: number;
  totalWords: number;
  totalDictations: number;
  averageWpm: number;
  wpmVsTypingPercent: number;
  averageLatencyMs: number;
  totalAudioMinutes: number;
  learnedCorrections: number;
  appUsage: AppUsageEntry[];
  dailyActivity: DailyActivityEntry[];
  recentLatency: RecentLatencyEntry[];
  streak: StreakInfo;
  timeSaved: EstimatedTimeSaved;
  hourAppHeatmap: HourAppHeatmapCell[];
}

interface DictionaryUpdatedPayload {
  added: string[];
  removed: string[];
  source?: "manual" | "learned";
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
    const unlistenLatency = listen<LatencyMetricsPayload>(
      "latency-metrics",
      () => {
        void loadInsights();
      },
    );
    const unlistenDictionary = listen<DictionaryUpdatedPayload>(
      "dictionary-updated",
      (event) => {
        const { added, removed, source } = event.payload;
        if (removed.length > 0) {
          void loadInsights();
          return;
        }
        if (added.length === 0 || source === "manual") {
          return;
        }
        setInsights((current) => {
          if (!current) {
            void loadInsights();
            return current;
          }
          return {
            ...current,
            learnedCorrections: current.learnedCorrections + added.length,
          };
        });
      },
    );

    return () => {
      cancelled = true;
      void unlistenHistory.then((drop) => drop());
      void unlistenLatency.then((drop) => drop());
      void unlistenDictionary.then((drop) => drop());
    };
  }, [loadInsights]);

  return { insights, loaded, errorMessage, reload: loadInsights };
}
