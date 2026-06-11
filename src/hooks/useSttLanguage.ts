import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { TFunction } from "i18next";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

export type SttLanguageCode = "fr" | "en" | "auto";

interface SttLanguageChangedPayload {
  language: string;
  detected: boolean;
}

const STT_LANGUAGE_KEYS: Record<SttLanguageCode, string> = {
  fr: "settings.sttLanguage.fr",
  en: "settings.sttLanguage.en",
  auto: "settings.sttLanguage.auto",
};

function isSttLanguageCode(value: string): value is SttLanguageCode {
  return value === "fr" || value === "en" || value === "auto";
}

export function sttLanguageLabel(code: string, t: TFunction): string {
  if (isSttLanguageCode(code)) {
    return t(STT_LANGUAGE_KEYS[code]);
  }
  return code.toUpperCase();
}

export function useSttLanguage() {
  const { t } = useTranslation();
  const [language, setLanguage] = useState<SttLanguageCode>("fr");
  const [cycling, setCycling] = useState(false);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      try {
        const value = await invoke<string>("get_stt_language");
        if (!cancelled && isSttLanguageCode(value)) {
          setLanguage(value);
        }
      } catch {
        // Backend not ready yet.
      }
    };

    void load();

    const unlisten = listen<SttLanguageChangedPayload>(
      "stt-language-changed",
      (event) => {
        const { language: next, detected } = event.payload;
        if (!isSttLanguageCode(next)) {
          return;
        }
        if (detected && next === "auto") {
          return;
        }
        setLanguage(next);
      },
    );

    return () => {
      cancelled = true;
      void unlisten.then((drop) => drop());
    };
  }, []);

  const cycleLanguage = useCallback(async () => {
    if (cycling) {
      return;
    }
    setCycling(true);
    try {
      const next = await invoke<string>("cycle_dictation_language");
      if (isSttLanguageCode(next)) {
        setLanguage(next);
      }
    } finally {
      setCycling(false);
    }
  }, [cycling]);

  return {
    language,
    languageLabel: sttLanguageLabel(language, t),
    cycling,
    cycleLanguage,
  };
}
