export type UiLanguageCode = "fr" | "en";

export function parseUiLanguage(value: string | undefined | null): UiLanguageCode {
  const normalized = value?.trim().toLowerCase();
  if (normalized === "en" || normalized?.startsWith("en-")) {
    return "en";
  }
  return "fr";
}

export function resolveUiLocale(code: string | undefined | null): UiLanguageCode {
  return parseUiLanguage(code);
}

export function toIntlLocale(code: UiLanguageCode): string {
  return code === "en" ? "en-US" : "fr-FR";
}
