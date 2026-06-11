import type { TFunction } from "i18next";
import type { DictationEntry } from "../../hooks/useHistory";

export type HistorySort = "recent" | "oldest" | "longest";

export function getHistorySortLabels(t: TFunction): Record<HistorySort, string> {
  return {
    recent: t("history.sort.recent"),
    oldest: t("history.sort.oldest"),
    longest: t("history.sort.longest"),
  };
}

export function getHistoryGroupLabels(t: TFunction) {
  return {
    today: t("common.today"),
    yesterday: t("common.yesterday"),
    thisWeek: t("common.thisWeek"),
    thisMonth: t("common.thisMonth"),
    other: t("common.other"),
  };
}

export type HistoryGroupLabels = ReturnType<typeof getHistoryGroupLabels>;

export const HISTORY_SORT_ORDER: HistorySort[] = [
  "recent",
  "oldest",
  "longest",
];

export function nextHistorySort(current: HistorySort): HistorySort {
  if (current === "recent") return "oldest";
  if (current === "oldest") return "longest";
  return "recent";
}

export function sortHistoryEntries(
  entries: DictationEntry[],
  sort: HistorySort,
): DictationEntry[] {
  const sorted = [...entries];
  switch (sort) {
    case "oldest":
      sorted.sort((a, b) => a.created_at.localeCompare(b.created_at));
      break;
    case "longest":
      sorted.sort((a, b) => b.wordCount - a.wordCount);
      break;
    case "recent":
    default:
      sorted.sort((a, b) => b.created_at.localeCompare(a.created_at));
      break;
  }
  return sorted;
}

export function formatEntryTime(iso: string, intlLocale: string): string {
  const date = parseEntryDate(iso);
  if (!date) {
    return iso;
  }
  return new Intl.DateTimeFormat(intlLocale, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export function formatEntryClock(iso: string, intlLocale: string): string {
  const date = parseEntryDate(iso);
  if (!date) {
    return "";
  }
  return new Intl.DateTimeFormat(intlLocale, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function parseEntryDate(iso: string): Date | null {
  const date = new Date(iso.includes("T") ? iso : `${iso.replace(" ", "T")}Z`);
  return Number.isNaN(date.getTime()) ? null : date;
}

function startOfDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

export function groupLabelForDate(
  iso: string,
  intlLocale: string,
  groupLabels: HistoryGroupLabels,
): string {
  const date = parseEntryDate(iso);
  if (!date) {
    return groupLabels.other;
  }

  const today = startOfDay(new Date());
  const entryDay = startOfDay(date);
  const diffDays = Math.round(
    (today.getTime() - entryDay.getTime()) / (1000 * 60 * 60 * 24),
  );

  if (diffDays === 0) return groupLabels.today;
  if (diffDays === 1) return groupLabels.yesterday;
  if (diffDays < 7) return groupLabels.thisWeek;
  if (diffDays < 30) return groupLabels.thisMonth;
  return new Intl.DateTimeFormat(intlLocale, {
    month: "long",
    year: "numeric",
  }).format(date);
}

export interface HistoryGroup {
  label: string;
  entries: DictationEntry[];
}

export function groupHistoryEntries(
  entries: DictationEntry[],
  intlLocale: string,
  groupLabels: HistoryGroupLabels,
): HistoryGroup[] {
  const groups = new Map<string, DictationEntry[]>();

  for (const entry of entries) {
    const label = groupLabelForDate(entry.created_at, intlLocale, groupLabels);
    const bucket = groups.get(label);
    if (bucket) {
      bucket.push(entry);
    } else {
      groups.set(label, [entry]);
    }
  }

  return Array.from(groups.entries()).map(([label, groupEntries]) => ({
    label,
    entries: groupEntries,
  }));
}

export function computeHistoryStats(entries: DictationEntry[]) {
  const totalWords = entries.reduce((sum, entry) => sum + entry.wordCount, 0);
  const totalLatency = entries.reduce((sum, entry) => sum + entry.totalMs, 0);
  const avgLatency =
    entries.length > 0 ? Math.round(totalLatency / entries.length) : 0;

  return {
    count: entries.length,
    totalWords,
    avgLatency,
  };
}

export function formatAppLabel(entry: DictationEntry): string | null {
  if (entry.appTitle?.trim()) {
    return entry.appTitle.trim();
  }
  if (entry.appExe?.trim()) {
    return entry.appExe.trim();
  }
  return null;
}
