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

export type HotkeyCaptureResult =
  | { action: "ignore" }
  | { action: "cancel" }
  | { action: "invalid" }
  | { action: "capture"; hotkey: string };

const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);

const SUPPORTED_HOTKEY_KEYS = new Set([
  "Space",
  "Enter",
  "Tab",
  "Backspace",
  "Escape",
  "F1",
  "F2",
  "F3",
  "F4",
  "F5",
  "F6",
  "F7",
  "F8",
  "F9",
  "F10",
  "F11",
  "F12",
  "0",
  "1",
  "2",
  "3",
  "4",
  "5",
  "6",
  "7",
  "8",
  "9",
  ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ",
]);

function normalizeHotkeyKey(key: string): string {
  if (key === " ") {
    return "Space";
  }
  if (key.length === 1) {
    return key.toUpperCase();
  }
  return key;
}

export function isHotkeyKeySupported(key: string): boolean {
  return SUPPORTED_HOTKEY_KEYS.has(normalizeHotkeyKey(key));
}

export function isHotkeySupported(hotkey: string): boolean {
  const parts = hotkeyParts(hotkey);
  if (parts.length < 2) {
    return false;
  }
  const keyPart = parts[parts.length - 1];
  const modifiers = parts.slice(0, -1);
  if (modifiers.length === 0) {
    return false;
  }
  const validModifiers = new Set(["Ctrl", "Alt", "Shift", "Super"]);
  return (
    modifiers.every((part) => validModifiers.has(part)) &&
    isHotkeyKeySupported(keyPart)
  );
}

export function captureHotkey(event: KeyboardEvent): HotkeyCaptureResult {
  event.preventDefault();
  event.stopPropagation();

  if (event.key === "Escape") {
    return { action: "cancel" };
  }

  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Super");

  const key = event.key;
  if (MODIFIER_KEYS.has(key)) {
    return { action: "ignore" };
  }

  const normalizedKey = normalizeHotkeyKey(key);

  if (parts.length === 0) {
    return { action: "invalid" };
  }

  if (!isHotkeyKeySupported(normalizedKey)) {
    return { action: "invalid" };
  }

  parts.push(normalizedKey);
  return { action: "capture", hotkey: parts.join("+") };
}
