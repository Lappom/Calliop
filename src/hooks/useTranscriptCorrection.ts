import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

export function useTranscriptCorrection(lastTranscript: string | null) {
  const [editedTranscript, setEditedTranscript] = useState("");
  const [originalTranscript, setOriginalTranscript] = useState<string | null>(null);
  const [learning, setLearning] = useState(false);
  const [learnedWords, setLearnedWords] = useState<string[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const applyingRef = useRef(false);

  useEffect(() => {
    if (lastTranscript === null) {
      return;
    }
    setOriginalTranscript(lastTranscript);
    setEditedTranscript(lastTranscript);
    setLearnedWords([]);
    setErrorMessage(null);
  }, [lastTranscript]);

  const applyCorrection = useCallback(async () => {
    if (applyingRef.current) {
      return [];
    }
    if (!originalTranscript || editedTranscript.trim() === originalTranscript.trim()) {
      return [];
    }

    applyingRef.current = true;
    setLearning(true);
    setErrorMessage(null);
    try {
      const added = await invoke<string[]>("learn_from_correction", {
        original: originalTranscript,
        corrected: editedTranscript,
      });
      setLearnedWords(added);
      setOriginalTranscript(editedTranscript);
      return added;
    } catch (err) {
      setErrorMessage(String(err));
      throw err;
    } finally {
      applyingRef.current = false;
      setLearning(false);
    }
  }, [editedTranscript, originalTranscript]);

  const hasChanges =
    originalTranscript !== null &&
    editedTranscript.trim() !== originalTranscript.trim();

  return {
    editedTranscript,
    setEditedTranscript,
    applyCorrection,
    learning,
    learnedWords,
    errorMessage,
    hasChanges,
  };
}
