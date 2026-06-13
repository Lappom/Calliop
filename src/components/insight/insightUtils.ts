import type { TFunction } from "i18next";
import type {
  DailyActivityEntry,
  HourAppHeatmapCell,
  Insights,
  LatencySnapshot,
} from "../../hooks/useInsights";
import type { LatencyMetricsPayload } from "../../hooks/usePipelineState";

export function formatLatencyDetail(
  latency: Pick<
    LatencyMetricsPayload,
    "sttMs" | "sttWaitMs" | "llmMs" | "llmBlockedMs" | "injectMs"
  >,
  t: TFunction,
): string {
  const sttMs =
    latency.sttWaitMs != null
      ? latency.sttWaitMs + latency.sttMs
      : latency.sttMs;
  const parts = [
    `${t("insight.charts.latency.stt")} ${sttMs} ms`,
    `${t("insight.charts.latency.inject")} ${latency.injectMs} ms`,
  ];
  if (latency.llmBlockedMs != null && latency.llmBlockedMs > 0) {
    parts.push(`${t("insight.charts.latency.llm")} ${latency.llmBlockedMs} ms`);
  } else if (latency.llmMs > 0) {
    parts.push(`${t("insight.charts.latency.llm")} ${latency.llmMs} ms`);
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

export function formatWeekdayLabel(isoDate: string, intlLocale: string): string {
  const date = new Date(`${isoDate}T12:00:00`);
  if (Number.isNaN(date.getTime())) {
    return isoDate;
  }
  return new Intl.DateTimeFormat(intlLocale, { weekday: "long" }).format(date);
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

export function formatAudioDuration(minutes: number, t: TFunction): string {
  if (minutes <= 0) {
    return t("common.emDash");
  }
  if (minutes < 60) {
    return `${minutes} ${t("common.minutesShort")}`;
  }
  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;
  return remainder > 0
    ? `${hours} ${t("common.hoursShort")} ${remainder} ${t("common.minutesShort")}`
    : `${hours} ${t("common.hoursShort")}`;
}

export function formatTimeSaved(minutes: number, t: TFunction): string {
  if (minutes <= 0) {
    return t("common.emDash");
  }
  if (minutes < 60) {
    return `${minutes} ${t("common.minutesShort")}`;
  }
  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;
  return remainder > 0
    ? `${hours} ${t("common.hoursShort")} ${remainder} ${t("common.minutesShort")}`
    : `${hours} ${t("common.hoursShort")}`;
}

export interface HourAppHeatmapMatrix {
  apps: string[];
  lookup: Map<string, number>;
  max: number;
}

const HEATMAP_APP_LIMIT = 6;

export function buildHourAppHeatmap(
  cells: HourAppHeatmapCell[],
): HourAppHeatmapMatrix {
  const appTotals = new Map<string, number>();
  const lookup = new Map<string, number>();
  let max = 0;

  for (const cell of cells) {
    appTotals.set(
      cell.exeName,
      (appTotals.get(cell.exeName) ?? 0) + cell.wordCount,
    );
    const key = `${cell.exeName}:${cell.hour}`;
    lookup.set(key, cell.wordCount);
    max = Math.max(max, cell.wordCount);
  }

  const apps = [...appTotals.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, HEATMAP_APP_LIMIT)
    .map(([name]) => name);

  return { apps, lookup, max };
}

export function hasHeatmapData(cells: HourAppHeatmapCell[]): boolean {
  return cells.some((cell) => cell.wordCount > 0);
}
