import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

export interface AppSettings {
  autoEdit: boolean;
  autoLearn: boolean;
}

interface SettingsPayload {
  auto_edit: boolean;
  auto_learn: boolean;
}

interface LlmModelDownloadProgress {
  downloaded: number;
  total: number | null;
  percent: number;
  source: string;
}

function toPayload(settings: AppSettings): SettingsPayload {
  return {
    auto_edit: settings.autoEdit,
    auto_learn: settings.autoLearn,
  };
}

function fromPayload(payload: SettingsPayload): AppSettings {
  return {
    autoEdit: payload.auto_edit,
    autoLearn: payload.auto_learn,
  };
}

export function useSettings() {
  const [settings, setSettings] = useState<AppSettings>({
    autoEdit: false,
    autoLearn: true,
  });
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [llmReady, setLlmReady] = useState(false);
  const [llmProgress, setLlmProgress] = useState<number | null>(null);
  const settingsRef = useRef(settings);
  const llmReadyRef = useRef(llmReady);
  const llmProgressRef = useRef(llmProgress);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      try {
        const payload = await invoke<SettingsPayload>("get_settings");
        if (!cancelled) {
          const loaded = fromPayload(payload);
          settingsRef.current = loaded;
          setSettings(loaded);
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
        llmReadyRef.current = true;
        llmProgressRef.current = null;
        setLlmReady(true);
        setLlmProgress(null);
      }),
      listen("llm-unready", () => {
        llmReadyRef.current = false;
        llmProgressRef.current = null;
        setLlmReady(false);
        setLlmProgress(null);
      }),
      listen<LlmModelDownloadProgress>("llm-model-download-progress", (event) => {
        llmProgressRef.current = event.payload.percent;
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
    const previousSettings = settingsRef.current;
    const previousLlmReady = llmReadyRef.current;
    const previousLlmProgress = llmProgressRef.current;
    settingsRef.current = next;
    setSettings(next);

    try {
      if (next.autoEdit) {
        llmProgressRef.current = 0;
        llmReadyRef.current = false;
        setLlmProgress(0);
        setLlmReady(false);
      } else {
        llmReadyRef.current = false;
        llmProgressRef.current = null;
        setLlmReady(false);
        setLlmProgress(null);
      }
      await invoke("set_settings", { settings: toPayload(next) });
      if (!next.autoEdit) {
        llmProgressRef.current = null;
        setLlmProgress(null);
      }
    } catch (err) {
      settingsRef.current = previousSettings;
      llmReadyRef.current = previousLlmReady;
      llmProgressRef.current = previousLlmProgress;
      setSettings(previousSettings);
      setLlmReady(previousLlmReady);
      setLlmProgress(previousLlmProgress);
      setErrorMessage(String(err));
      throw err;
    } finally {
      setSaving(false);
    }
  }, []);

  const setAutoEdit = useCallback(
    async (enabled: boolean) => {
      await saveSettings({ ...settingsRef.current, autoEdit: enabled });
    },
    [saveSettings],
  );

  const setAutoLearn = useCallback(
    async (enabled: boolean) => {
      await saveSettings({ ...settingsRef.current, autoLearn: enabled });
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
    setAutoLearn,
    saveSettings,
  };
}
