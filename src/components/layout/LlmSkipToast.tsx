import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { pickVariants, toastVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

interface LlmSkippedPayload {
  status: "skipped" | "failed";
  reason?: string | null;
}

const TOAST_DURATION_MS = 6_000;

export function LlmSkipToast() {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const variants = pickVariants(toastVariants, reducedMotion);
  const [payload, setPayload] = useState<LlmSkippedPayload | null>(null);

  useEffect(() => {
    let timeoutId: ReturnType<typeof setTimeout> | undefined;

    const unlisten = listen<LlmSkippedPayload>("llm-skipped", (event) => {
      setPayload(event.payload);
      if (timeoutId) {
        clearTimeout(timeoutId);
      }
      timeoutId = setTimeout(() => {
        setPayload(null);
      }, TOAST_DURATION_MS);
    });

    return () => {
      if (timeoutId) {
        clearTimeout(timeoutId);
      }
      void unlisten.then((drop) => drop());
    };
  }, []);

  if (!payload) {
    return null;
  }

  const reasonKey = payload.reason ?? "unknown";
  const message = t(`toasts.llmSkipped.reasons.${reasonKey}`, {
    defaultValue: t("toasts.llmSkipped.reasons.unknown"),
  });

  return (
    <div
      className="pointer-events-none fixed bottom-4 left-4 z-[60] flex w-[min(100vw-2rem,360px)] flex-col gap-3"
      aria-live="polite"
    >
      <AnimatePresence initial={false}>
        <motion.div
          key={`${payload.status}-${reasonKey}`}
          layout={!reducedMotion}
          variants={variants}
          initial="initial"
          animate="animate"
          exit="exit"
          className="pointer-events-auto rounded-lg border border-hairline-strong bg-surface-elevated p-4"
          role="status"
        >
          <p className="text-body-sm m-0 font-medium text-ink">
            {t("toasts.llmSkipped.title")}
          </p>
          <p className="text-caption m-0 mt-1 text-charcoal">{message}</p>
        </motion.div>
      </AnimatePresence>
    </div>
  );
}
