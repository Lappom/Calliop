import type { TFunction } from "i18next";

const ERROR_CODE_PATTERN = /^[A-Z][A-Z0-9_]*$/;

export function translateError(err: unknown, t: TFunction): string {
  const message = err instanceof Error ? err.message : String(err);
  const trimmed = message.trim();

  if (!ERROR_CODE_PATTERN.test(trimmed)) {
    return message;
  }

  const key = `errors.${trimmed}`;
  const translated = t(key);
  return translated === key ? message : translated;
}
