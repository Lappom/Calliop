import { X } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { pickVariants, toastVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

const TOAST_DURATION_MS = 6_000;

interface ErrorToastProps {
  message: string | null;
  title?: string;
  onDismiss: () => void;
}

export function ErrorToast({ message, title, onDismiss }: ErrorToastProps) {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const variants = pickVariants(toastVariants, reducedMotion);

  useEffect(() => {
    if (!message) {
      return;
    }
    const timeoutId = setTimeout(onDismiss, TOAST_DURATION_MS);
    return () => clearTimeout(timeoutId);
  }, [message, onDismiss]);

  return (
    <div
      className="pointer-events-none fixed bottom-4 right-4 z-[61] flex w-[min(100vw-2rem,360px)] flex-col gap-3"
      aria-live="assertive"
    >
      <AnimatePresence initial={false}>
        {message ? (
          <motion.div
            key={message}
            layout={!reducedMotion}
            variants={variants}
            initial="initial"
            animate="animate"
            exit="exit"
            className="pointer-events-auto rounded-lg border border-accent-red/35 bg-surface-elevated p-4 shadow-[0_0_24px_rgba(255,32,71,0.12)]"
            role="alert"
          >
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-body-sm m-0 font-medium text-ink">
                  {title ?? t("toasts.error.title")}
                </p>
                <p className="text-caption m-0 mt-1 text-charcoal">{message}</p>
              </div>
              <button
                type="button"
                onClick={onDismiss}
                className={[
                  "shrink-0 rounded-md p-1 text-charcoal",
                  "transition-colors duration-150",
                  "hover:bg-surface-card hover:text-ink",
                  "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-hairline-strong",
                ].join(" ")}
                aria-label={t("window.close")}
              >
                <X size={16} strokeWidth={2} aria-hidden />
              </button>
            </div>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
}
