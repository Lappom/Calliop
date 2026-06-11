export type AppView =
  | "main"
  | "dictionary"
  | "snippets"
  | "style"
  | "history"
  | "insight"
  | "settings";

export const PRIMARY_VIEWS: { id: AppView; label: string }[] = [
  { id: "main", label: "Accueil" },
  { id: "dictionary", label: "Dictionnaire" },
  { id: "snippets", label: "Snippets" },
  { id: "style", label: "Style" },
  { id: "history", label: "Historique" },
  { id: "insight", label: "Statistiques" },
];

export const BOTTOM_VIEWS: { id: AppView; label: string }[] = [
  { id: "settings", label: "Paramètres" },
];
