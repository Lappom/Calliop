import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

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
      listen("llm-unready", () => {
        setLlmReady(false);
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
    const previousSettings = settings;
    const previousLlmReady = llmReady;
    const previousLlmProgress = llmProgress;
    setSettings(next);

    try {
      if (next.autoEdit) {
        setLlmProgress(0);
        setLlmReady(false);
      } else {
        setLlmReady(false);
        setLlmProgress(null);
      }
      await invoke("set_settings", { settings: toPayload(next) });
      if (!next.autoEdit) {
        setLlmProgress(null);
      }
    } catch (err) {
      setSettings(previousSettings);
      setLlmReady(previousLlmReady);
      setLlmProgress(previousLlmProgress);
      setErrorMessage(String(err));
      throw err;
    } finally {
      setSaving(false);
    }
  }, [settings, llmReady, llmProgress]);

  const setAutoEdit = useCallback(
    async (enabled: boolean) => {
      await saveSettings({ ...settings, autoEdit: enabled });
    },
    [saveSettings, settings],
  );

  const setAutoLearn = useCallback(
    async (enabled: boolean) => {
      await saveSettings({ ...settings, autoLearn: enabled });
    },
    [saveSettings, settings],
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
