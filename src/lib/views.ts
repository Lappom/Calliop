import type { TFunction } from "i18next";

export type AppView =
  | "main"
  | "dictionary"
  | "snippets"
  | "style"
  | "history"
  | "insight"
  | "achievements";

export function isAppView(value: string): value is AppView {
  return (
    value === "main" ||
    value === "dictionary" ||
    value === "snippets" ||
    value === "style" ||
    value === "history" ||
    value === "insight" ||
    value === "achievements"
  );
}

export function isSettingsNavigation(value: string): boolean {
  return value === "settings";
}

const PRIMARY_VIEW_IDS: AppView[] = [
  "main",
  "history",
  "insight",
  "achievements",
  "dictionary",
  "snippets",
  "style",
];

export function getPrimaryViews(t: TFunction): { id: AppView; label: string }[] {
  return PRIMARY_VIEW_IDS.map((id) => ({
    id,
    label: t(`nav.items.${id}`),
  }));
}
