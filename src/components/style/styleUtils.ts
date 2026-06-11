import type { TFunction } from "i18next";
import type {
  ActiveWindow,
  AppContextMatchType,
  AppContextRule,
  ToneProfile,
} from "../../hooks/useAppContext";

export const TONE_PROFILES: ToneProfile[] = [
  "default",
  "casual",
  "formal",
  "technical",
];

export interface ToneMeta {
  label: string;
  description: string;
  accent: string;
}

export function getMatchTypeLabels(
  t: TFunction,
): Record<AppContextMatchType, string> {
  return {
    exe: t("style.matchType.exe"),
    title_contains: t("style.matchType.titleContains"),
  };
}

export function getToneMeta(t: TFunction): Record<ToneProfile, ToneMeta> {
  return {
    default: {
      label: t("style.tone.default.label"),
      description: t("style.tone.default.description"),
      accent: "var(--color-charcoal)",
    },
    casual: {
      label: t("style.tone.casual.label"),
      description: t("style.tone.casual.description"),
      accent: "var(--color-accent-green)",
    },
    formal: {
      label: t("style.tone.formal.label"),
      description: t("style.tone.formal.description"),
      accent: "var(--color-accent-blue)",
    },
    technical: {
      label: t("style.tone.technical.label"),
      description: t("style.tone.technical.description"),
      accent: "var(--color-accent-orange)",
    },
  };
}

export type StyleRuleSort = "pattern-asc" | "pattern-desc" | "tone" | "recent";

export function getStyleSortLabels(
  t: TFunction,
): Record<StyleRuleSort, string> {
  return {
    "pattern-asc": t("style.sort.patternAsc"),
    "pattern-desc": t("style.sort.patternDesc"),
    tone: t("style.sort.tone"),
    recent: t("style.sort.recent"),
  };
}

function normalizeExePattern(pattern: string): string {
  const trimmed = pattern.trim();
  const base = trimmed.split(/[/\\]/).pop() ?? trimmed;
  const lower = base.toLowerCase();
  return lower.endsWith(".exe") ? lower : `${lower}.exe`;
}

function exeNamesMatch(pattern: string, windowExe: string): boolean {
  return normalizeExePattern(pattern) === normalizeExePattern(windowExe);
}

export function ruleMatchesWindow(
  window: ActiveWindow,
  rule: AppContextRule,
): boolean {
  if (rule.matchType === "exe") {
    return exeNamesMatch(rule.pattern, window.exeName);
  }
  return window.title
    .toLowerCase()
    .includes(rule.pattern.toLowerCase());
}

export function resolveMatchingRule(
  rules: AppContextRule[],
  window: ActiveWindow | null,
): AppContextRule | null {
  if (!window) {
    return null;
  }
  return rules.find((rule) => ruleMatchesWindow(window, rule)) ?? null;
}

export function resolveActiveTone(
  rules: AppContextRule[],
  window: ActiveWindow | null,
): ToneProfile {
  return resolveMatchingRule(rules, window)?.tone ?? "default";
}

export function filterStyleRules(
  rules: AppContextRule[],
  query: string,
  t: TFunction,
): AppContextRule[] {
  const toneMeta = getToneMeta(t);
  const matchTypeLabels = getMatchTypeLabels(t);
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return rules;
  }
  return rules.filter((rule) => {
    const toneLabel = toneMeta[rule.tone].label.toLowerCase();
    const matchLabel = matchTypeLabels[rule.matchType].toLowerCase();
    return (
      rule.pattern.toLowerCase().includes(normalized) ||
      toneLabel.includes(normalized) ||
      matchLabel.includes(normalized)
    );
  });
}

export function sortStyleRules(
  rules: AppContextRule[],
  sort: StyleRuleSort,
  t: TFunction,
  intlLocale: string,
): AppContextRule[] {
  const toneMeta = getToneMeta(t);
  const sorted = [...rules];
  switch (sort) {
    case "pattern-desc":
      sorted.sort((a, b) =>
        b.pattern.localeCompare(a.pattern, intlLocale, { sensitivity: "base" }),
      );
      break;
    case "tone":
      sorted.sort((a, b) =>
        toneMeta[a.tone].label.localeCompare(toneMeta[b.tone].label, intlLocale),
      );
      break;
    case "recent":
      sorted.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
      break;
    case "pattern-asc":
    default:
      sorted.sort((a, b) =>
        a.pattern.localeCompare(b.pattern, intlLocale, { sensitivity: "base" }),
      );
      break;
  }
  return sorted;
}

export function nextStyleSort(current: StyleRuleSort): StyleRuleSort {
  if (current === "pattern-asc") return "pattern-desc";
  if (current === "pattern-desc") return "tone";
  if (current === "tone") return "recent";
  return "pattern-asc";
}

export const STYLE_SORT_ORDER: StyleRuleSort[] = [
  "pattern-asc",
  "pattern-desc",
  "tone",
  "recent",
];
