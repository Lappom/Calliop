import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import type { SttLanguageCode } from "./useSttLanguage";

export type WhisperModelId = "small" | "distil-fr-dec16";
export type LlmModelId = "qwen3-0.6b" | "qwen3-1.7b" | "qwen3-4b";
export type InferenceBackendId = "auto" | "cpu";

export interface AppSettings {
  autoEdit: boolean;
  autoLearn: boolean;
  autoUpdate: boolean;
  sttLanguage: SttLanguageCode;
  whisperModel: WhisperModelId;
  llmModel: LlmModelId;
  hotkey: string;
  inferenceBackend: InferenceBackendId;
}

export interface ModelStatusEntry {
  id: string;
  label: string;
  installed: boolean;
  size_bytes: number | null;
  active: boolean;
}

export interface ModelsStatus {
  whisper: ModelStatusEntry[];
  llm: ModelStatusEntry[];
}

export interface InferenceInfo {
  compiled_backend: string;
  gpu_available: boolean;
  active_backend: string;
  inference_backend_setting: string;
}

interface SettingsPayload {
  auto_edit: boolean;
  auto_learn: boolean;
  auto_update: boolean;
  stt_language: string;
  whisper_model: string;
  llm_model: string;
  hotkey: string;
  inference_backend: string;
}

interface DownloadProgress {
  model_id: string;
  downloaded: number;
  total: number | null;
  percent: number;
  source: string;
}

const WHISPER_MODEL_IDS = ["small", "distil-fr-dec16"] as const;
const LLM_MODEL_IDS = ["qwen3-0.6b", "qwen3-1.7b", "qwen3-4b"] as const;

function parseWhisperModel(value: string): WhisperModelId {
  if (value === "medium") {
    return "distil-fr-dec16";
  }
  return WHISPER_MODEL_IDS.includes(value as WhisperModelId)
    ? (value as WhisperModelId)
    : "small";
}

function parseLlmModel(value: string): LlmModelId {
  return LLM_MODEL_IDS.includes(value as LlmModelId)
    ? (value as LlmModelId)
    : "qwen3-1.7b";
}

function parseInferenceBackend(value: string): InferenceBackendId {
  return value === "cpu" ? "cpu" : "auto";
}

function toPayload(settings: AppSettings): SettingsPayload {
  return {
    auto_edit: settings.autoEdit,
    auto_learn: settings.autoLearn,
    auto_update: settings.autoUpdate,
    stt_language: settings.sttLanguage,
    whisper_model: settings.whisperModel,
    llm_model: settings.llmModel,
    hotkey: settings.hotkey,
    inference_backend: settings.inferenceBackend,
  };
}

function fromPayload(payload: SettingsPayload): AppSettings {
  const sttLanguage =
    payload.stt_language === "en" || payload.stt_language === "auto"
      ? payload.stt_language
      : "fr";
  return {
    autoEdit: payload.auto_edit,
    autoLearn: payload.auto_learn,
    autoUpdate: payload.auto_update,
    sttLanguage,
    whisperModel: parseWhisperModel(payload.whisper_model),
    llmModel: parseLlmModel(payload.llm_model),
    hotkey: payload.hotkey,
    inferenceBackend: parseInferenceBackend(payload.inference_backend),
  };
}

export const DEFAULT_SETTINGS: AppSettings = {
  autoEdit: true,
  autoLearn: true,
  autoUpdate: false,
  sttLanguage: "fr",
  whisperModel: "small",
  llmModel: "qwen3-1.7b",
  hotkey: "Alt+Space",
  inferenceBackend: "auto",
};

function formatBytes(bytes: number | null): string {
  if (bytes === null) return "—";
  if (bytes >= 1_000_000_000) {
    return `${(bytes / 1_000_000_000).toFixed(1)} Go`;
  }
  return `${Math.round(bytes / 1_000_000)} Mo`;
}

export function useSettings() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [llmReady, setLlmReady] = useState(false);
  const [llmProgress, setLlmProgress] = useState<number | null>(null);
  const [llmProgressModel, setLlmProgressModel] = useState<string | null>(null);
  const [sttProgress, setSttProgress] = useState<number | null>(null);
  const [sttProgressModel, setSttProgressModel] = useState<string | null>(null);
  const [modelsStatus, setModelsStatus] = useState<ModelsStatus | null>(null);
  const [inferenceInfo, setInferenceInfo] = useState<InferenceInfo | null>(null);
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const settingsRef = useRef(settings);
  const llmReadyRef = useRef(llmReady);
  const llmProgressRef = useRef(llmProgress);

  const refreshModelsStatus = useCallback(async () => {
    const status = await invoke<ModelsStatus>("get_models_status");
    setModelsStatus(status);
  }, []);

  const refreshInferenceInfo = useCallback(async () => {
    const info = await invoke<InferenceInfo>("get_inference_info");
    setInferenceInfo(info);
  }, []);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      try {
        const [payload, autostart, info] = await Promise.all([
          invoke<SettingsPayload>("get_settings"),
          invoke<boolean>("is_autostart_enabled"),
          invoke<InferenceInfo>("get_inference_info"),
        ]);
        if (!cancelled) {
          const loadedSettings = fromPayload(payload);
          settingsRef.current = loadedSettings;
          setSettings(loadedSettings);
          setAutostartEnabled(autostart);
          setInferenceInfo(info);
          setLoaded(true);
          void refreshModelsStatus();
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
        setLlmProgressModel(null);
        void refreshModelsStatus();
      }),
      listen("llm-unready", () => {
        llmReadyRef.current = false;
        llmProgressRef.current = null;
        setLlmReady(false);
        setLlmProgress(null);
        setLlmProgressModel(null);
      }),
      listen<DownloadProgress>("llm-model-download-progress", (event) => {
        llmProgressRef.current = event.payload.percent;
        setLlmProgress(event.payload.percent);
        setLlmProgressModel(event.payload.model_id);
      }),
      listen<DownloadProgress>("model-download-progress", (event) => {
        setSttProgress(event.payload.percent);
        setSttProgressModel(event.payload.model_id);
      }),
      listen("model-ready", () => {
        setSttProgress(null);
        setSttProgressModel(null);
        void refreshModelsStatus();
      }),
    ]);

    return () => {
      cancelled = true;
      void unlisteners.then((drops) => drops.forEach((drop) => drop()));
    };
  }, [refreshModelsStatus, refreshInferenceInfo]);

  const saveSettings = useCallback(
    async (next: AppSettings) => {
      setSaving(true);
      setErrorMessage(null);
      const previousSettings = settingsRef.current;
      const previousLlmReady = llmReadyRef.current;
      const previousLlmProgress = llmProgressRef.current;
      settingsRef.current = next;
      setSettings(next);

      try {
        const autoEditChanged = next.autoEdit !== previousSettings.autoEdit;
        const llmModelChanged = next.llmModel !== previousSettings.llmModel;

        if (autoEditChanged) {
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
            setLlmProgressModel(null);
          }
        } else if (llmModelChanged && next.autoEdit) {
          llmProgressRef.current = 0;
          llmReadyRef.current = false;
          setLlmProgress(0);
          setLlmReady(false);
        }

        const whisperChanged =
          next.whisperModel !== previousSettings.whisperModel ||
          next.inferenceBackend !== previousSettings.inferenceBackend;
        if (whisperChanged) {
          setSttProgress(0);
        }

        await invoke("set_settings", { settings: toPayload(next) });

        if (autoEditChanged && !next.autoEdit) {
          llmProgressRef.current = null;
          setLlmProgress(null);
          setLlmProgressModel(null);
        }

        await Promise.all([refreshModelsStatus(), refreshInferenceInfo()]);
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
    },
    [refreshInferenceInfo, refreshModelsStatus],
  );

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

  const setAutoUpdate = useCallback(
    async (enabled: boolean) => {
      await saveSettings({ ...settingsRef.current, autoUpdate: enabled });
    },
    [saveSettings],
  );

  const setSttLanguage = useCallback(
    async (sttLanguage: AppSettings["sttLanguage"]) => {
      await saveSettings({ ...settingsRef.current, sttLanguage });
    },
    [saveSettings],
  );

  const setWhisperModel = useCallback(
    async (whisperModel: WhisperModelId) => {
      await saveSettings({ ...settingsRef.current, whisperModel });
    },
    [saveSettings],
  );

  const setLlmModel = useCallback(
    async (llmModel: LlmModelId) => {
      await saveSettings({ ...settingsRef.current, llmModel });
    },
    [saveSettings],
  );

  const setInferenceBackend = useCallback(
    async (inferenceBackend: InferenceBackendId) => {
      await saveSettings({ ...settingsRef.current, inferenceBackend });
    },
    [saveSettings],
  );

  const setHotkey = useCallback(async (hotkey: string) => {
    setSaving(true);
    setErrorMessage(null);
    const previous = settingsRef.current.hotkey;
    try {
      await invoke("set_hotkey", { hotkey });
      const next = { ...settingsRef.current, hotkey };
      settingsRef.current = next;
      setSettings(next);
    } catch (err) {
      setErrorMessage(String(err));
      settingsRef.current = { ...settingsRef.current, hotkey: previous };
      setSettings((s) => ({ ...s, hotkey: previous }));
      throw err;
    } finally {
      setSaving(false);
    }
  }, []);

  const setAutostart = useCallback(async (enabled: boolean) => {
    await invoke("set_autostart_enabled", { enabled });
    setAutostartEnabled(enabled);
  }, []);

  const resetSettings = useCallback(async () => {
    await saveSettings(DEFAULT_SETTINGS);
  }, [saveSettings]);

  const deleteModel = useCallback(
    async (kind: "whisper" | "llm", modelId: string) => {
      await invoke("delete_model", { kind, modelId });
      await refreshModelsStatus();
    },
    [refreshModelsStatus],
  );

  return {
    settings,
    loaded,
    saving,
    errorMessage,
    llmReady,
    llmProgress,
    llmProgressModel,
    sttProgress,
    sttProgressModel,
    modelsStatus,
    inferenceInfo,
    autostartEnabled,
    formatBytes,
    setAutoEdit,
    setAutoLearn,
    setAutoUpdate,
    setSttLanguage,
    setWhisperModel,
    setLlmModel,
    setInferenceBackend,
    setHotkey,
    setAutostart,
    resetSettings,
    deleteModel,
    refreshModelsStatus,
    saveSettings,
  };
}
