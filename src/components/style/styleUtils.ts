import type {
  ActiveWindow,
  AppContextMatchType,
  AppContextRule,
  ToneProfile,
} from "../../hooks/useAppContext";

export const MATCH_TYPE_LABELS: Record<AppContextMatchType, string> = {
  exe: "Exécutable",
  title_contains: "Titre contient",
};

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

export const TONE_META: Record<ToneProfile, ToneMeta> = {
  default: {
    label: "Neutre",
    description: "Transcription fidèle, sans reformulation stylistique.",
    accent: "var(--color-charcoal)",
  },
  casual: {
    label: "Décontracté",
    description: "Ton naturel et conversationnel pour le chat et les notes.",
    accent: "var(--color-accent-green)",
  },
  formal: {
    label: "Formel",
    description: "Registre professionnel pour e-mails et documents.",
    accent: "var(--color-accent-blue)",
  },
  technical: {
    label: "Technique",
    description: "Précision et concision pour le code et la doc.",
    accent: "var(--color-accent-orange)",
  },
};

export type StyleRuleSort = "pattern-asc" | "pattern-desc" | "tone" | "recent";

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
): AppContextRule[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) {
    return rules;
  }
  return rules.filter((rule) => {
    const toneLabel = TONE_META[rule.tone].label.toLowerCase();
    const matchLabel = MATCH_TYPE_LABELS[rule.matchType].toLowerCase();
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
): AppContextRule[] {
  const sorted = [...rules];
  switch (sort) {
    case "pattern-desc":
      sorted.sort((a, b) =>
        b.pattern.localeCompare(a.pattern, "fr", { sensitivity: "base" }),
      );
      break;
    case "tone":
      sorted.sort((a, b) =>
        TONE_META[a.tone].label.localeCompare(TONE_META[b.tone].label, "fr"),
      );
      break;
    case "recent":
      sorted.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
      break;
    case "pattern-asc":
    default:
      sorted.sort((a, b) =>
        a.pattern.localeCompare(b.pattern, "fr", { sensitivity: "base" }),
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

export const STYLE_SORT_LABELS: Record<StyleRuleSort, string> = {
  "pattern-asc": "Tri A → Z",
  "pattern-desc": "Tri Z → A",
  tone: "Par ton",
  recent: "Plus récents",
};

export const STYLE_SORT_ORDER: StyleRuleSort[] = [
  "pattern-asc",
  "pattern-desc",
  "tone",
  "recent",
];
