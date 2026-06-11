import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState, type ChangeEvent } from "react";
import { useTranslation } from "react-i18next";
import { translateError } from "../lib/translateError";

export interface Snippet {
  id: number;
  trigger: string;
  content: string;
  created_at: string;
}

export function useSnippets() {
  const { t } = useTranslation();
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [userName, setUserName] = useState("");
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
      setErrorMessage(translateError(err, t));
      setLoaded(true);
    }
  }, [t]);

  const loadUserName = useCallback(async () => {
    try {
      const name = await invoke<string>("get_snippet_user_name");
      setUserName(name);
    } catch (err) {
      setErrorMessage(translateError(err, t));
    }
  }, [t]);

  const saveUserName = useCallback(async (name: string) => {
    const trimmed = name.trim();
    try {
      await invoke("set_snippet_user_name", { name: trimmed });
      setUserName(trimmed);
      setErrorMessage(null);
    } catch (err) {
      setErrorMessage(translateError(err, t));
      throw err;
    }
  }, [t]);

  const previewExpansion = useCallback(async (content: string) => {
    try {
      return await invoke<string>("preview_snippet_expansion", { content });
    } catch (err) {
      setErrorMessage(translateError(err, t));
      return content;
    }
  }, [t]);

  useEffect(() => {
    let cancelled = false;

    const setup = async () => {
      if (!cancelled) {
        await Promise.all([loadSnippets(), loadUserName()]);
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
  }, [loadSnippets, loadUserName]);

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
          setErrorMessage(t("snippets.errors.duplicateTrigger"));
        }
        return inserted;
      } catch (err) {
        setErrorMessage(translateError(err, t));
        throw err;
      } finally {
        setBusy(false);
      }
    },
    [loadSnippets, t],
  );

  const removeSnippet = useCallback(
    async (id: number) => {
      setBusy(true);
      setErrorMessage(null);
      try {
        await invoke("remove_snippet", { id });
        await loadSnippets();
      } catch (err) {
        setErrorMessage(translateError(err, t));
        throw err;
      } finally {
        setBusy(false);
      }
    },
    [loadSnippets, t],
  );

  const updateSnippet = useCallback(
    async (id: number, trigger: string, content: string) => {
      const trimmedTrigger = trigger.trim();
      const trimmedContent = content.trim();
      if (!trimmedTrigger || !trimmedContent) {
        return false;
      }

      setBusy(true);
      setErrorMessage(null);
      try {
        const updated = await invoke<boolean>("update_snippet", {
          id,
          trigger: trimmedTrigger,
          content: trimmedContent,
        });
        if (updated) {
          await loadSnippets();
        } else {
          setErrorMessage(t("snippets.errors.duplicateTrigger"));
        }
        return updated;
      } catch (err) {
        setErrorMessage(translateError(err, t));
        throw err;
      } finally {
        setBusy(false);
      }
    },
    [loadSnippets, t],
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
        setErrorMessage(translateError(err, t));
        throw err;
      } finally {
        setBusy(false);
      }
    },
    [loadSnippets, t],
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
      setErrorMessage(translateError(err, t));
      throw err;
    } finally {
      setBusy(false);
    }
  }, [t]);

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
    userName,
    loaded,
    busy,
    errorMessage,
    fileInputRef,
    addSnippet,
    updateSnippet,
    removeSnippet,
    importSnippets,
    exportSnippets,
    openImportDialog,
    handleImportFile,
    saveUserName,
    previewExpansion,
    reload: loadSnippets,
  };
}
