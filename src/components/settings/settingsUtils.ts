import type { TFunction } from "i18next";

export type SettingsSectionId = "general" | "models" | "shortcuts" | "advanced";

export function getSettingsSections(t: TFunction): {
  id: SettingsSectionId;
  label: string;
  description: string;
}[] {
  return [
    {
      id: "general",
      label: t("settings.sections.general.label"),
      description: t("settings.sections.general.description"),
    },
    {
      id: "models",
      label: t("settings.sections.models.label"),
      description: t("settings.sections.models.description"),
    },
    {
      id: "shortcuts",
      label: t("settings.sections.shortcuts.label"),
      description: t("settings.sections.shortcuts.description"),
    },
    {
      id: "advanced",
      label: t("settings.sections.advanced.label"),
      description: t("settings.sections.advanced.description"),
    },
  ];
}

export function settingsSectionDomId(id: SettingsSectionId): string {
  return `settings-section-${id}`;
}

export function hotkeyParts(hotkey: string): string[] {
  return hotkey.split("+").map((part) => part.trim());
}

export function formatHotkeyLabel(hotkey: string, t: TFunction): string {
  return hotkey.replace(/Space/g, t("keys.space"));
}

export function hotkeyPartLabel(t: TFunction, part: string): string {
  if (part === "Space") {
    return t("keys.space");
  }
  const modifierKeys: Record<string, string> = {
    Ctrl: "keys.modifiers.ctrl",
    Alt: "keys.modifiers.alt",
    Shift: "keys.modifiers.shift",
    Super: "keys.modifiers.super",
  };
  const key = modifierKeys[part];
  return key ? t(key) : part;
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
