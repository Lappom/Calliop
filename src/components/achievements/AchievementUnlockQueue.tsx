import { AnimatePresence, motion } from "motion/react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AchievementUnlockedPayload } from "../../hooks/useAchievements";
import { pickVariants, toastVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";

const TOAST_DURATION_MS = 4_000;

const tierBorderClass: Record<AchievementUnlockedPayload["tier"], string> = {
  common: "border-hairline-strong",
  rare: "border-accent-orange/60 shadow-[0_0_24px_rgba(255,128,31,0.18)]",
  legendary: "border-accent-green/60 shadow-[0_0_28px_rgba(17,255,153,0.2)]",
};

export function AchievementUnlockQueue() {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const variants = pickVariants(toastVariants, reducedMotion);
  const queueRef = useRef<AchievementUnlockedPayload[]>([]);
  const [toast, setToast] = useState<AchievementUnlockedPayload | null>(null);
  const [legendary, setLegendary] = useState<AchievementUnlockedPayload | null>(
    null,
  );
  const processingRef = useRef(false);

  const markSeen = useCallback(async (id: string) => {
    try {
      await invoke("mark_achievements_seen", { ids: [id] });
    } catch {
      // non-blocking
    }
  }, []);

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
    window.setTimeout(() => {
      void markSeen(next.id);
      setToast(null);
      processingRef.current = false;
      processQueue();
    }, TOAST_DURATION_MS);
  }, [markSeen]);

  useEffect(() => {
    const unlisten = listen<AchievementUnlockedPayload>(
      "achievement-unlocked",
      (event) => {
        queueRef.current.push(event.payload);
        processQueue();
      },
    );
    return () => {
      void unlisten.then((drop) => drop());
    };
  }, [processQueue]);

  const dismissLegendary = useCallback(() => {
    if (legendary) {
      void markSeen(legendary.id);
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
                "pointer-events-auto rounded-lg border bg-surface-elevated p-4",
                tierBorderClass[toast.tier],
              ].join(" ")}
              role="status"
            >
              <p className="text-body-sm m-0 font-medium text-ink">
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
        >
          <div className="flex flex-col items-center gap-3 text-center">
            <p className="text-heading-sm m-0 font-medium text-accent-green">
              {renderAchievementText(legendary).title}
            </p>
            <p className="text-body-sm m-0 text-charcoal">
              {renderAchievementText(legendary).description}
            </p>
            <Button type="button" onClick={dismissLegendary} className="mt-2">
              {t("achievements.celebration.dismiss")}
            </Button>
          </div>
        </Modal>
      )}
    </>
  );
}
