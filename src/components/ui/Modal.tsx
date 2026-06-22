import { AnimatePresence, motion } from "motion/react";
import {
  useCallback,
  useEffect,
  useId,
  useRef,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import {
  modalBackdropVariants,
  modalPanelVariants,
  pickVariants,
} from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

type ModalSize = "sm" | "md" | "lg" | "full";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: ReactNode;
  size?: ModalSize;
  hideHeader?: boolean;
  panelClassName?: string;
  backdropClassName?: string;
}

const sizeClasses: Record<ModalSize, string> = {
  sm: "max-w-sm",
  md: "max-w-md",
  lg: "max-w-lg",
  full: "w-[min(95vw,960px)] sm:w-[min(90vw,960px)]",
};

export function Modal({
  open,
  onClose,
  title,
  description,
  children,
  size = "md",
  hideHeader = false,
  panelClassName = "",
  backdropClassName = "",
}: ModalProps) {
  const titleId = useId();
  const descriptionId = useId();
  const panelRef = useRef<HTMLDivElement>(null);
  const reducedMotion = useReducedMotion();
  const backdropVariants = pickVariants(modalBackdropVariants, reducedMotion);
  const panelVariants = pickVariants(modalPanelVariants, reducedMotion);

  const handleBackdropClick = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      if (event.target === event.currentTarget) {
        onClose();
      }
    },
    [onClose],
  );

  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);

    const focusTimer = window.setTimeout(() => {
      const focusable = panelRef.current?.querySelector<HTMLElement>(
        'input:not([disabled]), textarea:not([disabled]), button:not([disabled]), select:not([disabled])',
      );
      focusable?.focus();
    }, 0);

    return () => {
      window.clearTimeout(focusTimer);
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [open, onClose]);

  const defaultPanelClass =
    size === "full"
      ? "overflow-hidden rounded-lg border border-hairline-strong bg-surface-elevated"
      : "w-[calc(100%-2rem)] rounded-lg border border-hairline-strong bg-surface-elevated p-6";

  return createPortal(
    <AnimatePresence>
      {open && (
        <motion.div
          className={[
            "fixed inset-0 z-50 flex items-center justify-center p-4",
            backdropClassName || "bg-black/70",
          ].join(" ")}
          variants={backdropVariants}
          initial="initial"
          animate="animate"
          exit="exit"
          onClick={handleBackdropClick}
          role="presentation"
        >
          <motion.div
            ref={panelRef}
            role="dialog"
            aria-modal="true"
            aria-labelledby={titleId}
            aria-describedby={description ? descriptionId : undefined}
            variants={panelVariants}
            initial="initial"
            animate="animate"
            exit="exit"
            className={[defaultPanelClass, sizeClasses[size], panelClassName]
              .filter(Boolean)
              .join(" ")}
            onClick={(event) => event.stopPropagation()}
          >
            {hideHeader ? (
              <h2 id={titleId} className="sr-only">
                {title}
              </h2>
            ) : (
              <header className="mb-6">
                <h2 id={titleId} className="text-heading-sm m-0 text-ink">
                  {title}
                </h2>
                {description && (
                  <p id={descriptionId} className="text-body-sm mt-2 text-charcoal">
                    {description}
                  </p>
                )}
              </header>
            )}
            {hideHeader && description && (
              <p id={descriptionId} className="sr-only">
                {description}
              </p>
            )}
            {children}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>,
    document.body,
  );
}
