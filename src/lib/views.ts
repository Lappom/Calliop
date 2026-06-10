export type AppView = "main" | "settings" | "onboarding";

export const APP_VIEWS: { id: AppView; label: string }[] = [
  { id: "main", label: "Accueil" },
  { id: "settings", label: "Réglages" },
  { id: "onboarding", label: "Guide" },
];
