import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import {
  parseUiLanguage,
  toIntlLocale,
  type UiLanguageCode,
} from "./locale";

export function useUiLocale() {
  const { t, i18n } = useTranslation();
  const locale: UiLanguageCode = parseUiLanguage(i18n.language);
  const intlLocale = toIntlLocale(locale);

  const formatDate = useCallback(
    (
      value: Date | number | string,
      options?: Intl.DateTimeFormatOptions,
    ): string => {
      const date = value instanceof Date ? value : new Date(value);
      if (Number.isNaN(date.getTime())) {
        return String(value);
      }
      return new Intl.DateTimeFormat(intlLocale, options).format(date);
    },
    [intlLocale],
  );

  const formatNumber = useCallback(
    (value: number, options?: Intl.NumberFormatOptions): string => {
      return new Intl.NumberFormat(intlLocale, options).format(value);
    },
    [intlLocale],
  );

  const formatBytes = useCallback(
    (bytes: number | null): string => {
      if (bytes === null) {
        return "—";
      }
      if (bytes >= 1_000_000_000) {
        return `${formatNumber(bytes / 1_000_000_000, {
          maximumFractionDigits: 1,
        })} ${t("common.units.gb")}`;
      }
      return `${formatNumber(Math.round(bytes / 1_000_000), {
        maximumFractionDigits: 0,
      })} ${t("common.units.mb")}`;
    },
    [formatNumber, t],
  );

  return {
    locale,
    intlLocale,
    formatDate,
    formatNumber,
    formatBytes,
    t,
    i18n,
  };
}
