import type {
  DailyActivityEntry,
  Insights,
  LatencySnapshot,
} from "../../hooks/useInsights";
import type { LatencyMetricsPayload } from "../../hooks/usePipelineState";

export function formatLatencyDetail(
  latency: Pick<
    LatencyMetricsPayload,
    "sttMs" | "sttWaitMs" | "llmMs" | "llmBlockedMs" | "injectMs"
  >,
): string {
  const sttLabel =
    latency.sttWaitMs != null
      ? `STT ${latency.sttWaitMs + latency.sttMs} ms`
      : `STT ${latency.sttMs} ms`;
  const parts = [sttLabel, `injection ${latency.injectMs} ms`];
  if (latency.llmBlockedMs != null && latency.llmBlockedMs > 0) {
    parts.push(`LLM ${latency.llmBlockedMs} ms`);
  } else if (latency.llmMs > 0) {
    parts.push(`LLM ${latency.llmMs} ms`);
  }
  return parts.join(" · ");
}

export function resolveActiveLatency(
  sessionLatency: LatencyMetricsPayload | null,
  storedLatency: LatencySnapshot | null | undefined,
): LatencyMetricsPayload | null {
  if (sessionLatency) {
    return sessionLatency;
  }
  if (!storedLatency) {
    return null;
  }
  return {
    sttMs: storedLatency.sttMs,
    llmMs: storedLatency.llmMs,
    injectMs: storedLatency.injectMs,
    totalMs: storedLatency.totalMs,
  };
}

export interface WeekSummary {
  totalWords: number;
  totalDictations: number;
  averageWordsPerDay: number;
  bestDay: DailyActivityEntry | null;
}

export function computeWeekSummary(
  dailyActivity: DailyActivityEntry[],
): WeekSummary {
  const totalWords = dailyActivity.reduce(
    (sum, day) => sum + day.wordCount,
    0,
  );
  const totalDictations = dailyActivity.reduce(
    (sum, day) => sum + day.dictationCount,
    0,
  );
  const activeDays = dailyActivity.filter((day) => day.wordCount > 0).length;
  const averageWordsPerDay =
    activeDays > 0 ? Math.round(totalWords / activeDays) : 0;
  const bestDay =
    dailyActivity.reduce<DailyActivityEntry | null>((best, day) => {
      if (!best || day.wordCount > best.wordCount) {
        return day;
      }
      return best;
    }, null) ?? null;

  return {
    totalWords,
    totalDictations,
    averageWordsPerDay,
    bestDay,
  };
}

export function formatWeekdayLabel(isoDate: string): string {
  const date = new Date(`${isoDate}T12:00:00`);
  if (Number.isNaN(date.getTime())) {
    return isoDate;
  }
  return new Intl.DateTimeFormat("fr-FR", { weekday: "long" }).format(date);
}

export function hasInsightData(insights: Insights | null): boolean {
  if (!insights) {
    return false;
  }
  return (
    insights.totalWords > 0 ||
    insights.totalDictations > 0 ||
    insights.learnedCorrections > 0 ||
    insights.lastLatency !== null
  );
}

export function formatAudioDuration(minutes: number): string {
  if (minutes <= 0) {
    return "—";
  }
  if (minutes < 60) {
    return `${minutes} min`;
  }
  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;
  return remainder > 0 ? `${hours} h ${remainder} min` : `${hours} h`;
}
