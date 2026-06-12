import { invoke } from "@tauri-apps/api/core";
import { useCallback, useMemo, useState } from "react";
import type { PipelineState } from "./usePipelineState";

export type PracticePhase = "idle" | "recording" | "processing" | "success" | "error";

function derivePracticePhase(
  pipelineState: string,
  practiceText: string,
  pipelineError: string | null,
): PracticePhase {
  if (pipelineError || pipelineState === "error") {
    return "error";
  }
  if (practiceText.trim().length > 0) {
    return "success";
  }
  if (pipelineState === "recording") {
    return "recording";
  }
  if (pipelineState === "transcribing" || pipelineState === "injecting") {
    return "processing";
  }
  return "idle";
}

interface UseOnboardingPracticeOptions {
  pipelineState: string;
  pipelineError: string | null;
}

export function useOnboardingPractice({
  pipelineState,
  pipelineError,
}: UseOnboardingPracticeOptions) {
  const [practiceText, setPracticeText] = useState("");
  const [partialTranscript, setPartialTranscript] = useState("");
  const [preparing, setPreparing] = useState(false);
  const [prepareError, setPrepareError] = useState<string | null>(null);

  const practicePhase = useMemo(
    () => derivePracticePhase(pipelineState, practiceText, pipelineError),
    [pipelineState, practiceText, pipelineError],
  );

  const practiceReady = practiceText.trim().length > 0;

  const enterPracticeStep = useCallback(async () => {
    setPreparing(true);
    setPrepareError(null);
    setPartialTranscript("");
    try {
      await invoke("prepare_onboarding_dictation");
    } catch (err) {
      setPrepareError(String(err));
    } finally {
      setPreparing(false);
    }
  }, []);

  const exitPracticeStep = useCallback(async () => {
    setPartialTranscript("");
    try {
      await invoke("prepare_onboarding_dictation");
    } catch {
      // Best-effort cleanup when leaving the practice step.
    }
  }, []);

  const syncPracticeFromField = useCallback((value: string) => {
    setPracticeText(value);
  }, []);

  const onPartialTranscript = useCallback((text: string) => {
    setPartialTranscript(text);
  }, []);

  const onPipelineIdle = useCallback((message: string | null | undefined) => {
    setPartialTranscript("");
    if (message) {
      setPracticeText((current) => {
        if (current.includes(message)) {
          return current;
        }
        if (!current.trim()) {
          return message;
        }
        const separator = current.endsWith(" ") ? "" : " ";
        return `${current}${separator}${message}`;
      });
    }
  }, []);

  return {
    practiceText,
    practicePhase,
    practiceReady,
    partialTranscript,
    preparing,
    prepareError,
    enterPracticeStep,
    exitPracticeStep,
    syncPracticeFromField,
    onPartialTranscript,
    onPipelineIdle,
    setPracticeText,
  };
}

export function isActivePipelineState(state: string): state is PipelineState {
  return state === "recording" || state === "transcribing" || state === "injecting";
}
