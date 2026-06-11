import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

export function useTranscriptCorrection(
  lastTranscript: string | null,
  transcriptRevision: number,
) {
  const [editedTranscript, setEditedTranscript] = useState("");
  const [originalTranscript, setOriginalTranscript] = useState<string | null>(null);
  const [learning, setLearning] = useState(false);
  const [learnedWords, setLearnedWords] = useState<string[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const applyingRef = useRef(false);
  const editedRef = useRef(editedTranscript);
  const originalRef = useRef(originalTranscript);
  const learnQueueRef = useRef<Promise<string[]>>(Promise.resolve([]));

  editedRef.current = editedTranscript;
  originalRef.current = originalTranscript;

  const enqueueLearn = useCallback((original: string, edited: string) => {
    if (edited.trim() === original.trim()) {
      return Promise.resolve([] as string[]);
    }

    const next = learnQueueRef.current
      .catch(() => [] as string[])
      .then(async () => {
        applyingRef.current = true;
        setLearning(true);
        setErrorMessage(null);
        try {
          const added = await invoke<string[]>("learn_from_correction", {
            original,
            corrected: edited,
          });
          setLearnedWords(added);
          setOriginalTranscript(edited);
          originalRef.current = edited;
          editedRef.current = edited;
          return added;
        } catch (err) {
          setErrorMessage(String(err));
          throw err;
        } finally {
          applyingRef.current = false;
          setLearning(false);
        }
      });

    learnQueueRef.current = next;
    return next;
  }, []);

  const applyCorrection = useCallback(async () => {
    if (!originalRef.current) {
      return [];
    }
    return enqueueLearn(originalRef.current, editedRef.current);
  }, [enqueueLearn]);

  useEffect(() => {
    if (lastTranscript === null) {
      return;
    }

    const syncTranscript = async () => {
      const original = originalRef.current;
      const edited = editedRef.current;
      let learnFailed = false;

      if (original !== null && edited.trim() !== original.trim()) {
        try {
          await enqueueLearn(original, edited);
        } catch {
          learnFailed = true;
        }
      } else {
        try {
          await learnQueueRef.current;
        } catch {
          // Ignore in-flight manual correction failures when syncing.
        }
      }

      setOriginalTranscript(lastTranscript);
      setEditedTranscript(lastTranscript);
      originalRef.current = lastTranscript;
      editedRef.current = lastTranscript;
      setLearnedWords([]);
      if (!learnFailed) {
        setErrorMessage(null);
      }
    };

    void syncTranscript();
  }, [lastTranscript, transcriptRevision, enqueueLearn]);

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
