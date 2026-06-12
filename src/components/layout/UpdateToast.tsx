import { AnimatePresence, motion } from "motion/react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { pickVariants, toastVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import { Button } from "../ui/Button";

interface UpdateReadyPayload {
  version: string;
}

export function UpdateToast() {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const variants = pickVariants(toastVariants, reducedMotion);
  const [version, setVersion] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    void invoke<string | null>("get_pending_update_version")
      .then((pendingVersion) => {
        if (!cancelled && pendingVersion) {
          setVersion(pendingVersion);
        }
      })
      .catch(() => {});

    const unlisten = listen<UpdateReadyPayload>("update-ready", (event) => {
      setVersion(event.payload.version);
      setError(null);
    });

    return () => {
      cancelled = true;
      void unlisten.then((drop) => drop());
    };
  }, []);

  const handleInstall = useCallback(async () => {
    if (installing) {
      return;
    }
    setInstalling(true);
    setError(null);
    try {
      await invoke("install_pending_update");
    } catch (err) {
      setInstalling(false);
      setError(typeof err === "string" ? err : t("toasts.appUpdate.installError"));
    }
  }, [installing, t]);

  if (!version) {
    return null;
  }

  return (
    <div
      className="pointer-events-none fixed bottom-4 right-4 z-[60] flex w-[min(100vw-2rem,320px)] flex-col items-stretch gap-3"
      aria-live="polite"
    >
      <AnimatePresence initial={false}>
        <motion.div
          key={version}
          layout={!reducedMotion}
          variants={variants}
          initial="initial"
          animate="animate"
          exit="exit"
          className="pointer-events-auto rounded-lg border border-hairline-strong bg-surface-elevated p-4"
          role="status"
        >
          <p className="text-body-sm m-0 font-medium text-ink">
            {t("toasts.appUpdate.title", { version })}
          </p>
          <p className="text-caption m-0 mt-1 text-charcoal">
            {t("toasts.appUpdate.description")}
          </p>
          {error ? (
            <p className="text-caption m-0 mt-2 text-accent-red">{error}</p>
          ) : null}
          <div className="mt-3 flex gap-2">
            <Button
              variant="ghost"
              className="flex-1"
              disabled={installing}
              onClick={() => {
                void invoke("dismiss_pending_update").then(() => {
                  setVersion(null);
                  setError(null);
                });
              }}
            >
              {t("toasts.appUpdate.later")}
            </Button>
            <Button
              className="flex-1"
              disabled={installing}
              onClick={() => {
                void handleInstall();
              }}
            >
              {installing
                ? t("toasts.appUpdate.installing")
                : t("toasts.appUpdate.install")}
            </Button>
          </div>
        </motion.div>
      </AnimatePresence>
    </div>
  );
}
