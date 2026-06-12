import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

export function useMicProbe() {
  const [audioLevel, setAudioLevel] = useState(0);
  const [micProbing, setMicProbing] = useState(false);
  const [micProbeStopping, setMicProbeStopping] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void listen<{ level: number }>("audio-level", (event) => {
      setAudioLevel(event.payload.level);
    }).then((drop) => {
      unlisten = drop;
    });

    return () => {
      unlisten?.();
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
    setMicProbeStopping(true);
    try {
      await invoke("stop_mic_probe");
      setMicProbing(false);
      setAudioLevel(0);
    } finally {
      setMicProbeStopping(false);
    }
  }, []);

  return {
    audioLevel,
    micProbing,
    micProbeStopping,
    startMicProbe,
    stopMicProbe,
  };
}
