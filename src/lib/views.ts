export type AppView =
  | "main"
  | "dictionary"
  | "snippets"
  | "context"
  | "history"
  | "insight"
  | "settings";

export const PRIMARY_VIEWS: { id: AppView; label: string }[] = [
  { id: "main", label: "Accueil" },
  { id: "dictionary", label: "Dictionnaire" },
  { id: "snippets", label: "Snippets" },
  { id: "context", label: "Contexte" },
  { id: "history", label: "Historique" },
  { id: "insight", label: "Insight" },
];

export const BOTTOM_VIEWS: { id: AppView; label: string }[] = [
  { id: "settings", label: "Paramètres" },
];
