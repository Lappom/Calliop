import type { TFunction } from "i18next";

export type AppView =
  | "main"
  | "dictionary"
  | "snippets"
  | "style"
  | "history"
  | "insight"
  | "settings";

export function isAppView(value: string): value is AppView {
  return (
    value === "main" ||
    value === "dictionary" ||
    value === "snippets" ||
    value === "style" ||
    value === "history" ||
    value === "insight" ||
    value === "settings"
  );
}

const PRIMARY_VIEW_IDS: AppView[] = [
  "main",
  "history",
  "insight",
  "dictionary",
  "snippets",
  "style",
];

const BOTTOM_VIEW_IDS: AppView[] = ["settings"];

export function getPrimaryViews(t: TFunction): { id: AppView; label: string }[] {
  return PRIMARY_VIEW_IDS.map((id) => ({
    id,
    label: t(`nav.items.${id}`),
  }));
}

export function getBottomViews(t: TFunction): { id: AppView; label: string }[] {
  return BOTTOM_VIEW_IDS.map((id) => ({
    id,
    label: t(`nav.items.${id}`),
  }));
}
