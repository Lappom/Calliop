import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

type UpdateCheckResult =
  | { status: "upToDate" }
  | { status: "downloading"; version: string }
  | { status: "ready"; version: string }
  | { status: "unavailableInDev" };

interface UpdateReadyPayload {
  version: string;
}

interface UpdateDownloadProgressPayload {
  version: string;
  downloaded: number;
  total: number | null;
  percent: number;
}

interface UpdateCheckFailedPayload {
  message: string;
}

export type AppUpdateStatus =
  | "idle"
  | "checking"
  | "upToDate"
  | "downloading"
  | "ready"
  | "error"
  | "devUnavailable";

export function useAppUpdate() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<AppUpdateStatus>("idle");
  const [pendingVersion, setPendingVersion] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    let cancelled = false;

    void invoke<string | null>("get_pending_update_version")
      .then((version) => {
        if (!cancelled && version) {
          setPendingVersion(version);
          setStatus("ready");
        }
      })
      .catch(() => {});

    const unlistenReady = listen<UpdateReadyPayload>("update-ready", (event) => {
      setPendingVersion(event.payload.version);
      setDownloadProgress(null);
      setStatus("ready");
      setErrorMessage(null);
    });

    const unlistenProgress = listen<UpdateDownloadProgressPayload>(
      "update-download-progress",
      (event) => {
        setPendingVersion(event.payload.version);
        setDownloadProgress(Math.round(event.payload.percent));
        setStatus("downloading");
      },
    );

    const unlistenFailed = listen<UpdateCheckFailedPayload>(
      "update-check-failed",
      (event) => {
        setDownloadProgress(null);
        setStatus("error");
        setErrorMessage(event.payload.message);
      },
    );

    return () => {
      cancelled = true;
      void unlistenReady.then((drop) => drop());
      void unlistenProgress.then((drop) => drop());
      void unlistenFailed.then((drop) => drop());
    };
  }, []);

  const checkForUpdates = useCallback(async () => {
    setStatus("checking");
    setErrorMessage(null);
    setDownloadProgress(null);

    try {
      const result = await invoke<UpdateCheckResult>("check_for_updates");

      switch (result.status) {
        case "upToDate":
          setPendingVersion(null);
          setStatus("upToDate");
          break;
        case "downloading":
          setPendingVersion(result.version);
          setDownloadProgress(0);
          setStatus("downloading");
          break;
        case "ready":
          setPendingVersion(result.version);
          setStatus("ready");
          break;
        case "unavailableInDev":
          setStatus("devUnavailable");
          break;
      }
    } catch (err) {
      const message = typeof err === "string" ? err : t("settings.updatesPanel.error");
      if (message.includes("déjà en cours") || message.toLowerCase().includes("already")) {
        setErrorMessage(t("settings.updatesPanel.alreadyChecking"));
      } else {
        setErrorMessage(message);
      }
      setStatus("error");
    }
  }, [t]);

  const installUpdate = useCallback(async () => {
    if (installing) {
      return;
    }
    setInstalling(true);
    setErrorMessage(null);
    try {
      await invoke("install_pending_update");
    } catch (err) {
      setInstalling(false);
      setErrorMessage(
        typeof err === "string" ? err : t("settings.updatesPanel.installError"),
      );
    }
  }, [installing, t]);

  const isBusy = status === "checking" || status === "downloading" || installing;

  return {
    status,
    pendingVersion,
    downloadProgress,
    errorMessage,
    installing,
    isBusy,
    checkForUpdates,
    installUpdate,
  };
}
