export const INSIGHT_TAB_IDS = [
  "overview",
  "activity",
  "performance",
  "global",
] as const;

export type InsightTabId = (typeof INSIGHT_TAB_IDS)[number];

export function isInsightTabId(value: string): value is InsightTabId {
  return INSIGHT_TAB_IDS.includes(value as InsightTabId);
}

export function insightTabPanelId(tab: InsightTabId): string {
  return `insight-panel-${tab}`;
}

/** Restore tab from URL hash on load, e.g. #activity */
export function readInsightTabFromHash(): InsightTabId | null {
  if (typeof window === "undefined") {
    return null;
  }
  const hash = window.location.hash.replace(/^#/, "");
  return isInsightTabId(hash) ? hash : null;
}
