import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState, type ChangeEvent } from "react";

export interface Snippet {
  id: number;
  trigger: string;
  content: string;
  created_at: string;
}

export function useSnippets() {
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const loadSnippets = useCallback(async () => {
    try {
      const entries = await invoke<Snippet[]>("list_snippets");
      setSnippets(entries);
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
        await loadSnippets();
      }
    };

    void setup();

    const unlisten = listen("snippets-updated", () => {
      void loadSnippets();
    });

    return () => {
      cancelled = true;
      void unlisten.then((drop) => drop());
    };
  }, [loadSnippets]);

  const addSnippet = useCallback(
    async (trigger: string, content: string) => {
      const trimmedTrigger = trigger.trim();
      const trimmedContent = content.trim();
      if (!trimmedTrigger || !trimmedContent) {
        return false;
      }

      setBusy(true);
      setErrorMessage(null);
      try {
        const inserted = await invoke<boolean>("add_snippet", {
          trigger: trimmedTrigger,
          content: trimmedContent,
        });
        if (inserted) {
          await loadSnippets();
        } else {
          setErrorMessage("Ce déclencheur existe déjà.");
        }
        return inserted;
      } catch (err) {
        setErrorMessage(String(err));
        throw err;
      } finally {
        setBusy(false);
      }
    },
    [loadSnippets],
  );

  const removeSnippet = useCallback(
    async (id: number) => {
      setBusy(true);
      setErrorMessage(null);
      try {
        await invoke("remove_snippet", { id });
        await loadSnippets();
      } catch (err) {
        setErrorMessage(String(err));
        throw err;
      } finally {
        setBusy(false);
      }
    },
    [loadSnippets],
  );

  const importSnippets = useCallback(
    async (json: string) => {
      setBusy(true);
      setErrorMessage(null);
      try {
        const count = await invoke<number>("import_snippets", { json });
        await loadSnippets();
        return count;
      } catch (err) {
        setErrorMessage(String(err));
        throw err;
      } finally {
        setBusy(false);
      }
    },
    [loadSnippets],
  );

  const exportSnippets = useCallback(async () => {
    setBusy(true);
    setErrorMessage(null);
    try {
      const json = await invoke<string>("export_snippets");
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = "calliop-snippets.json";
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      setErrorMessage(String(err));
      throw err;
    } finally {
      setBusy(false);
    }
  }, []);

  const openImportDialog = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleImportFile = useCallback(
    async (event: ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0];
      event.target.value = "";
      if (!file) {
        return;
      }

      try {
        const json = await file.text();
        await importSnippets(json);
      } catch {
        // errorMessage is set in importSnippets
      }
    },
    [importSnippets],
  );

  return {
    snippets,
    loaded,
    busy,
    errorMessage,
    fileInputRef,
    addSnippet,
    removeSnippet,
    importSnippets,
    exportSnippets,
    openImportDialog,
    handleImportFile,
    reload: loadSnippets,
  };
}
