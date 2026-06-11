import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { translateError } from "../lib/translateError";

export type AppContextMatchType = "exe" | "title_contains";
export type ToneProfile = "default" | "casual" | "formal" | "technical";

export interface ActiveWindow {
  title: string;
  exeName: string;
  exePath?: string | null;
}

export interface AppContextRule {
  id: number;
  pattern: string;
  matchType: AppContextMatchType;
  tone: ToneProfile;
  createdAt: string;
}

export function useAppContext() {
  const { t } = useTranslation();
  const [rules, setRules] = useState<AppContextRule[]>([]);
  const [activeWindow, setActiveWindow] = useState<ActiveWindow | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const loadRules = useCallback(async () => {
    try {
      const entries = await invoke<AppContextRule[]>("list_app_context_rules");
      setRules(entries);
      setLoaded(true);
      setErrorMessage(null);
    } catch (err) {
      setErrorMessage(translateError(err, t));
      setLoaded(true);
    }
  }, [t]);

  const refreshActiveWindow = useCallback(async () => {
    try {
      const window = await invoke<ActiveWindow | null>("get_active_window");
      setActiveWindow(window);
    } catch (err) {
      setErrorMessage(translateError(err, t));
    }
  }, [t]);

  useEffect(() => {
    let cancelled = false;

    const setup = async () => {
      if (!cancelled) {
        await Promise.all([loadRules(), refreshActiveWindow()]);
      }
    };

    void setup();

    const unlisten = listen("app-context-updated", () => {
      void loadRules();
    });

    return () => {
      cancelled = true;
      void unlisten.then((drop) => drop());
    };
  }, [loadRules, refreshActiveWindow]);

  const addRule = useCallback(
    async (pattern: string, matchType: AppContextMatchType, tone: ToneProfile) => {
      const trimmed = pattern.trim();
      if (!trimmed) {
        return false;
      }

      setBusy(true);
      setErrorMessage(null);
      try {
        const inserted = await invoke<boolean>("add_app_context_rule", {
          pattern: trimmed,
          matchType,
          tone,
        });
        if (inserted) {
          await loadRules();
        } else {
          setErrorMessage(t("style.errors.cannotAdd"));
        }
        return inserted;
      } catch (err) {
        setErrorMessage(translateError(err, t));
        throw err;
      } finally {
        setBusy(false);
      }
    },
    [loadRules, t],
  );

  const removeRule = useCallback(
    async (id: number) => {
      setBusy(true);
      setErrorMessage(null);
      try {
        await invoke("remove_app_context_rule", { id });
        await loadRules();
      } catch (err) {
        setErrorMessage(translateError(err, t));
        throw err;
      } finally {
        setBusy(false);
      }
    },
    [loadRules, t],
  );

  return {
    rules,
    activeWindow,
    loaded,
    busy,
    errorMessage,
    addRule,
    removeRule,
    refreshActiveWindow,
    reload: loadRules,
  };
}
