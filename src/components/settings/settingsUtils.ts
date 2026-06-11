export type SettingsSectionId = "general" | "models" | "shortcuts" | "advanced";

export const SETTINGS_SECTIONS: {
  id: SettingsSectionId;
  label: string;
  description: string;
}[] = [
  {
    id: "general",
    label: "Général",
    description: "Langue, auto-édition IA et apprentissage des corrections.",
  },
  {
    id: "models",
    label: "Modèles",
    description: "Whisper, LLM et gestion des fichiers installés.",
  },
  {
    id: "shortcuts",
    label: "Raccourcis",
    description: "Combinaison globale pour démarrer une dictée.",
  },
  {
    id: "advanced",
    label: "Avancé",
    description: "Mises à jour, démarrage automatique et backend d'inférence.",
  },
];

export function settingsSectionDomId(id: SettingsSectionId): string {
  return `settings-section-${id}`;
}

export function hotkeyParts(hotkey: string): string[] {
  return hotkey.split("+").map((part) => part.trim());
}

export function formatHotkeyLabel(hotkey: string): string {
  return hotkey.replace(/Space/g, "Espace");
}

export function captureHotkey(event: KeyboardEvent): string | null {
  event.preventDefault();
  event.stopPropagation();

  if (event.key === "Escape") {
    return null;
  }

  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Super");

  const key = event.key;
  if (["Control", "Alt", "Shift", "Meta"].includes(key)) {
    return null;
  }

  const normalizedKey =
    key === " " ? "Space" : key.length === 1 ? key.toUpperCase() : key;

  if (parts.length === 0) {
    return null;
  }

  parts.push(normalizedKey);
  return parts.join("+");
}
