import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

export function useOnboarding() {
  const [loading, setLoading] = useState(true);
  const [done, setDone] = useState(true);
  const [modelProgress, setModelProgress] = useState<number | null>(null);
  const [modelReady, setModelReady] = useState(false);
  const [audioLevel, setAudioLevel] = useState(0);
  const [micProbing, setMicProbing] = useState(false);
  const [dictationText, setDictationText] = useState("");
  const [pipelineState, setPipelineState] = useState("idle");
  const [hotkey, setHotkey] = useState("Alt+Space");

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
        if (!onboardingDone) {
          try {
            await invoke("ensure_model");
            if (!cancelled) {
              setModelReady(true);
            }
          } catch {
            // model-ready listener or retry on next launch
          }
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
  }, []);

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
    await invoke("stop_mic_probe");
    setMicProbing(false);
  }, []);

  const runDictationTest = useCallback(async () => {
    setDictationText("");
    await invoke("toggle_dictation");
  }, []);

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
    audioLevel,
    micProbing,
    dictationText,
    pipelineState,
    hotkey,
    startMicProbe,
    stopMicProbe,
    runDictationTest,
    completeOnboarding,
  };
}
