import { AnimatePresence, motion } from "motion/react";
import { Sparkles, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import {
  useAchievements,
  type AchievementUnlockedPayload,
} from "../../hooks/useAchievements";
import {
  pickVariants,
  successPopVariants,
  toastVariants,
} from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import { glowSurfaceClasses } from "../layout/glowSurface";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { tierUnlockedBorderClass } from "./achievementTierStyles";

const TOAST_DURATION_MS: Record<"common" | "rare", number> = {
  common: 4_000,
  rare: 5_000,
};

export function AchievementUnlockQueue() {
  const { t } = useTranslation();
  const { markSeen } = useAchievements();
  const reducedMotion = useReducedMotion();
  const variants = pickVariants(toastVariants, reducedMotion);
  const successVariants = pickVariants(successPopVariants, reducedMotion);
  const queueRef = useRef<AchievementUnlockedPayload[]>([]);
  const [toast, setToast] = useState<AchievementUnlockedPayload | null>(null);
  const [legendary, setLegendary] = useState<AchievementUnlockedPayload | null>(
    null,
  );
  const processingRef = useRef(false);
  const timeoutRef = useRef<number | null>(null);

  const clearToastTimeout = useCallback(() => {
    if (timeoutRef.current != null) {
      window.clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  const finishToast = useCallback(
    (payload: AchievementUnlockedPayload) => {
      clearToastTimeout();
      void markSeen([payload.id]);
      setToast(null);
      processingRef.current = false;
      if (queueRef.current.length > 0) {
        processQueueRef.current();
      }
    },
    [clearToastTimeout, markSeen],
  );

  const processQueueRef = useRef<() => void>(() => {});

  const processQueue = useCallback(() => {
    if (processingRef.current) {
      return;
    }
    const next = queueRef.current.shift();
    if (!next) {
      return;
    }
    processingRef.current = true;
    if (next.tier === "legendary") {
      setLegendary(next);
      return;
    }
    setToast(next);
    const duration =
      next.tier === "rare" ? TOAST_DURATION_MS.rare : TOAST_DURATION_MS.common;
    timeoutRef.current = window.setTimeout(() => {
      finishToast(next);
    }, duration);
  }, [finishToast]);

  processQueueRef.current = processQueue;

  useEffect(() => {
    const unlisten = listen<AchievementUnlockedPayload>(
      "achievement-unlocked",
      (event) => {
        queueRef.current.push(event.payload);
        processQueue();
      },
    );
    return () => {
      clearToastTimeout();
      void unlisten.then((drop) => drop());
    };
  }, [processQueue, clearToastTimeout]);

  const dismissToast = useCallback(() => {
    if (toast) {
      finishToast(toast);
    }
  }, [toast, finishToast]);

  const dismissLegendary = useCallback(() => {
    if (legendary) {
      void markSeen([legendary.id]);
    }
    setLegendary(null);
    processingRef.current = false;
    processQueue();
  }, [legendary, markSeen, processQueue]);

  const renderAchievementText = (payload: AchievementUnlockedPayload) => {
    const title = t(`achievements.items.${payload.id}.title`);
    const description = t(`achievements.items.${payload.id}.description`);
    return { title, description };
  };

  const toastBorderClass =
    toast?.tier === "rare"
      ? tierUnlockedBorderClass.rare
      : tierUnlockedBorderClass.common;

  return (
    <>
      <div
        className="pointer-events-none fixed bottom-4 right-4 z-[60] flex w-[min(100vw-2rem,360px)] flex-col gap-3"
        aria-live="polite"
      >
        <AnimatePresence initial={false}>
          {toast && (
            <motion.div
              key={toast.id}
              layout={!reducedMotion}
              variants={variants}
              initial="initial"
              animate="animate"
              exit="exit"
              className={[
                "pointer-events-auto relative overflow-hidden rounded-lg border bg-surface-elevated p-4",
                toast.tier === "rare"
                  ? glowSurfaceClasses("orange", "normal")
                  : "",
                toastBorderClass,
              ].join(" ")}
              role="status"
            >
              <button
                type="button"
                onClick={dismissToast}
                className="absolute right-2 top-2 inline-flex size-7 items-center justify-center rounded-md text-charcoal transition-transform duration-150 ease-out hover:bg-surface-card hover:text-ink active:scale-[0.97]"
                aria-label={t("achievements.toast.dismiss")}
              >
                <X size={14} strokeWidth={1.5} />
              </button>
              <p className="text-body-sm m-0 pr-8 font-medium text-ink">
                {t("achievements.toast.unlocked")}
              </p>
              <p className="text-body-sm m-0 mt-1 text-ink">
                {renderAchievementText(toast).title}
              </p>
              <p className="text-caption m-0 mt-1 text-charcoal">
                {renderAchievementText(toast).description}
              </p>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {legendary && (
        <Modal
          open
          onClose={dismissLegendary}
          title={t("achievements.celebration.title")}
          description={t("achievements.celebration.subtitle")}
          size="sm"
          panelClassName={[
            glowSurfaceClasses("green", "slow"),
            tierUnlockedBorderClass.legendary,
          ].join(" ")}
        >
          <motion.div
            variants={successVariants}
            initial="initial"
            animate="animate"
            className="flex flex-col items-center gap-3 text-center"
          >
            <div className="flex size-12 items-center justify-center rounded-full border border-accent-green/40 bg-surface-elevated text-accent-green">
              <Sparkles size={22} strokeWidth={1.5} />
            </div>
            <p className="text-heading-sm m-0 font-medium text-accent-green">
              {renderAchievementText(legendary).title}
            </p>
            <p className="text-body-sm m-0 text-charcoal">
              {renderAchievementText(legendary).description}
            </p>
            <Button type="button" onClick={dismissLegendary} className="mt-2">
              {t("achievements.celebration.dismiss")}
            </Button>
          </motion.div>
        </Modal>
      )}
    </>
  );
}
