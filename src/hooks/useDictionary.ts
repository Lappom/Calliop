import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";

export type DictionarySource = "manual" | "learned";

export interface DictionaryWord {
  id: number;
  word: string;
  source: DictionarySource;
  created_at: string;
}

export function useDictionary() {
  const [words, setWords] = useState<DictionaryWord[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const loadWords = useCallback(async () => {
    try {
      const entries = await invoke<DictionaryWord[]>("list_dictionary_words");
      setWords(entries);
      setLoaded(true);
      setErrorMessage(null);
    } catch (err) {
      setErrorMessage(String(err));
    }
  }, []);

  useEffect(() => {
    let cancelled = false;

    const setup = async () => {
      if (!cancelled) {
        await loadWords();
      }
    };

    void setup();

    const unlisten = listen("dictionary-updated", () => {
      void loadWords();
    });

    return () => {
      cancelled = true;
      void unlisten.then((drop) => drop());
    };
  }, [loadWords]);

  const addWord = useCallback(
    async (word: string) => {
      const trimmed = word.trim();
      if (!trimmed) {
        return false;
      }

      setBusy(true);
      setErrorMessage(null);
      try {
        const inserted = await invoke<boolean>("add_dictionary_word", { word: trimmed });
        if (inserted) {
          await loadWords();
        } else {
          setErrorMessage("Ce mot est déjà dans le dictionnaire.");
        }
        return inserted;
      } catch (err) {
        setErrorMessage(String(err));
        throw err;
      } finally {
        setBusy(false);
      }
    },
    [loadWords],
  );

  const removeWord = useCallback(
    async (id: number) => {
      setBusy(true);
      setErrorMessage(null);
      try {
        await invoke("remove_dictionary_word", { id });
        await loadWords();
      } catch (err) {
        setErrorMessage(String(err));
        throw err;
      } finally {
        setBusy(false);
      }
    },
    [loadWords],
  );

  return {
    words,
    loaded,
    busy,
    errorMessage,
    addWord,
    removeWord,
    reload: loadWords,
  };
}
