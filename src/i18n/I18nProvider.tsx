import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState, type ReactNode } from "react";
import { I18nextProvider } from "react-i18next";
import i18n from "./index";
import { parseUiLanguage } from "./locale";

interface SettingsPayload {
  ui_language?: string;
}

interface UiLanguageChangedPayload {
  language: string;
}

async function resolveInitialLanguage(): Promise<string> {
  if (!isTauri()) {
    return "fr";
  }

  try {
    const payload = await invoke<SettingsPayload>("get_settings");
    return parseUiLanguage(payload.ui_language);
  } catch {
    return "fr";
  }
}

interface I18nProviderProps {
  children: ReactNode;
}

export function I18nProvider({ children }: I18nProviderProps) {
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      const language = await resolveInitialLanguage();
      if (cancelled) {
        return;
      }
      await i18n.changeLanguage(language);
      if (!cancelled) {
        setReady(true);
      }
    };

    void load();

    const unlisten = isTauri()
      ? listen<UiLanguageChangedPayload | string>("ui-language-changed", (event) => {
          const next =
            typeof event.payload === "string"
              ? event.payload
              : event.payload.language;
          void i18n.changeLanguage(parseUiLanguage(next));
        })
      : Promise.resolve(() => {});

    return () => {
      cancelled = true;
      void unlisten.then((drop) => drop());
    };
  }, []);

  if (!ready) {
    return null;
  }

  return <I18nextProvider i18n={i18n}>{children}</I18nextProvider>;
}
