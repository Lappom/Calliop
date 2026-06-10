import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

export interface AppSettings {
  autoEdit: boolean;
}

interface SettingsPayload {
  auto_edit: boolean;
}

interface LlmModelDownloadProgress {
  downloaded: number;
  total: number | null;
  percent: number;
  source: string;
}

function toPayload(settings: AppSettings): SettingsPayload {
  return { auto_edit: settings.autoEdit };
}

function fromPayload(payload: SettingsPayload): AppSettings {
  return { autoEdit: payload.auto_edit };
}

export function useSettings() {
  const [settings, setSettings] = useState<AppSettings>({ autoEdit: false });
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [llmReady, setLlmReady] = useState(false);
  const [llmProgress, setLlmProgress] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      try {
        const payload = await invoke<SettingsPayload>("get_settings");
        if (!cancelled) {
          setSettings(fromPayload(payload));
          setLoaded(true);
        }
      } catch (err) {
        if (!cancelled) {
          setErrorMessage(String(err));
        }
      }
    };

    void load();

    const unlisteners = Promise.all([
      listen("llm-ready", () => {
        setLlmReady(true);
        setLlmProgress(null);
      }),
      listen<LlmModelDownloadProgress>("llm-model-download-progress", (event) => {
        setLlmProgress(event.payload.percent);
      }),
    ]);

    return () => {
      cancelled = true;
      void unlisteners.then((drops) => drops.forEach((drop) => drop()));
    };
  }, []);

  const saveSettings = useCallback(async (next: AppSettings) => {
    setSaving(true);
    setErrorMessage(null);
    setSettings(next);

    try {
      if (next.autoEdit) {
        setLlmProgress(0);
      }
      await invoke("set_settings", { settings: toPayload(next) });
      if (next.autoEdit) {
        setLlmReady(true);
        setLlmProgress(null);
      } else {
        setLlmReady(false);
        setLlmProgress(null);
      }
    } catch (err) {
      setErrorMessage(String(err));
      throw err;
    } finally {
      setSaving(false);
    }
  }, []);

  const setAutoEdit = useCallback(
    async (enabled: boolean) => {
      await saveSettings({ autoEdit: enabled });
    },
    [saveSettings],
  );

  return {
    settings,
    loaded,
    saving,
    errorMessage,
    llmReady,
    llmProgress,
    setAutoEdit,
    saveSettings,
  };
}
