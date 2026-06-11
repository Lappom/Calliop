import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

export function useOnboarding() {
  const [loading, setLoading] = useState(true);
  const [done, setDone] = useState(true);
  const [modelProgress, setModelProgress] = useState<number | null>(null);
  const [modelReady, setModelReady] = useState(false);
  const [modelLoading, setModelLoading] = useState(false);
  const [modelError, setModelError] = useState<string | null>(null);
  const [audioLevel, setAudioLevel] = useState(0);
  const [micProbing, setMicProbing] = useState(false);
  const [micProbeStopping, setMicProbeStopping] = useState(false);
  const [dictationText, setDictationText] = useState("");
  const [pipelineState, setPipelineState] = useState("idle");
  const [hotkey, setHotkey] = useState("Alt+Space");

  const ensureModelForOnboarding = useCallback(async () => {
    setModelLoading(true);
    setModelError(null);
    try {
      await invoke("ensure_model");
      setModelReady(true);
      setModelProgress(null);
    } catch (err) {
      setModelReady(false);
      setModelError(String(err));
    } finally {
      setModelLoading(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;

    const init = async () => {
      try {
        const [onboardingDone, settings] = await Promise.all([
          invoke<boolean>("is_onboarding_done"),
          invoke<{
            hotkey: string;
          }>("get_settings"),
        ]);
        if (!cancelled) {
          setDone(onboardingDone);
          setHotkey(settings.hotkey);
          setLoading(false);
        }
        if (!onboardingDone && !cancelled) {
          await ensureModelForOnboarding();
        }
      } catch {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    void init();

    const unlisteners = Promise.all([
      listen<{ percent: number; model_id: string }>(
        "model-download-progress",
        (event) => {
          setModelProgress(event.payload.percent);
        },
      ),
      listen("model-ready", () => {
        setModelReady(true);
        setModelProgress(null);
        setModelError(null);
      }),
      listen<string>("model-init-error", (event) => {
        setModelReady(false);
        setModelError(event.payload);
      }),
      listen<{ level: number }>("audio-level", (event) => {
        setAudioLevel(event.payload.level);
      }),
      listen<{ state: string; message?: string | null }>(
        "pipeline-state",
        (event) => {
          setPipelineState(event.payload.state);
          if (event.payload.state === "idle" && event.payload.message) {
            setDictationText(event.payload.message);
          }
        },
      ),
      listen<{ text: string }>("partial-transcript", (event) => {
        setDictationText(event.payload.text);
      }),
    ]);

    return () => {
      cancelled = true;
      void invoke("stop_mic_probe").catch(() => {});
      void unlisteners.then((drops) => drops.forEach((drop) => drop()));
    };
  }, [ensureModelForOnboarding]);

  const startMicProbe = useCallback(async () => {
    setMicProbing(true);
    setAudioLevel(0);
    try {
      await invoke("start_mic_probe");
    } catch {
      setMicProbing(false);
    }
  }, []);

  const stopMicProbe = useCallback(async () => {
    setMicProbeStopping(true);
    try {
      await invoke("stop_mic_probe");
      setMicProbing(false);
    } finally {
      setMicProbeStopping(false);
    }
  }, []);

  const runDictationTest = useCallback(async () => {
    if (micProbing || micProbeStopping) {
      await stopMicProbe();
    }
    setDictationText("");
    await invoke("toggle_dictation");
  }, [micProbing, micProbeStopping, stopMicProbe]);

  const completeOnboarding = useCallback(async () => {
    await invoke("stop_mic_probe").catch(() => {});
    setMicProbing(false);
    await invoke("set_onboarding_done", { done: true });
    setDone(true);
  }, []);

  return {
    loading,
    done,
    modelProgress,
    modelReady,
    modelLoading,
    modelError,
    audioLevel,
    micProbing,
    micProbeStopping,
    dictationText,
    pipelineState,
    hotkey,
    retryEnsureModel: ensureModelForOnboarding,
    startMicProbe,
    stopMicProbe,
    runDictationTest,
    completeOnboarding,
  };
}
