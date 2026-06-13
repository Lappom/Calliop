import { AnimatePresence, motion } from "motion/react";
import { X } from "lucide-react";
import { useCallback, useEffect, useId, useRef } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import {
  modalBackdropVariants,
  modalPanelVariants,
  pickVariants,
} from "../../lib/motion/variants";
import { SettingsNav } from "./SettingsNav";
import { SettingsView } from "./SettingsView";
import type { SettingsSectionId } from "./settingsUtils";
import { settingsSectionDomId } from "./settingsUtils";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
  activeSection: SettingsSectionId;
  onSectionChange: (section: SettingsSectionId) => void;
}

export function SettingsModal({
  open,
  onClose,
  activeSection,
  onSectionChange,
}: SettingsModalProps) {
  const { t } = useTranslation();
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
      panelRef.current?.querySelector<HTMLElement>("button, input, select")?.focus();
    }, 0);

    return () => {
      window.clearTimeout(focusTimer);
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [open, onClose]);

  return createPortal(
    <AnimatePresence>
      {open && (
        <motion.div
          className="fixed inset-0 z-50 flex items-center justify-center bg-canvas/80 p-3 sm:p-4"
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
            aria-describedby={descriptionId}
            variants={panelVariants}
            initial="initial"
            animate="animate"
            exit="exit"
            className={[
              "flex h-[min(90vh,720px)] w-[min(95vw,960px)] flex-col overflow-hidden",
              "rounded-lg border border-hairline-strong bg-surface-elevated",
              "sm:h-[min(85vh,720px)] sm:w-[min(90vw,960px)]",
            ].join(" ")}
            onClick={(event) => event.stopPropagation()}
          >
            <header className="relative shrink-0 overflow-hidden border-b border-hairline px-6 py-5">
              <div
                className="pointer-events-none absolute inset-x-0 top-0 h-32"
                style={{
                  background:
                    "radial-gradient(ellipse 80% 100% at 50% 0%, var(--color-accent-blue-glow) 0%, transparent 70%)",
                  opacity: 0.08,
                }}
                aria-hidden
              />
              <div className="relative flex items-start justify-between gap-4">
                <div className="min-w-0">
                  <h2
                    id={titleId}
                    className="text-display-serif m-0 text-2xl text-ink"
                  >
                    {t("settings.title")}
                  </h2>
                  <p id={descriptionId} className="text-body-sm mt-2 text-charcoal">
                    {t("settings.subtitle")}
                  </p>
                </div>
                <button
                  type="button"
                  onClick={onClose}
                  className="inline-flex size-9 shrink-0 items-center justify-center rounded-md border border-hairline-strong bg-surface-card text-ink transition-[border-color] duration-150 hover:border-ink/30"
                  aria-label={t("settings.modal.aria.close")}
                >
                  <X
                    size={20}
                    strokeWidth={1.5}
                    absoluteStrokeWidth
                    aria-hidden
                  />
                </button>
              </div>
            </header>

            <div className="relative flex min-h-0 flex-1 flex-col lg:flex-row">
              <SettingsNav
                activeSection={activeSection}
                onSectionChange={onSectionChange}
              />

              <div
                className="flex min-h-0 min-w-0 flex-1 flex-col"
                role="tabpanel"
                id={`${settingsSectionDomId(activeSection)}-panel`}
                aria-labelledby={`${settingsSectionDomId(activeSection)}-tab`}
                aria-label={t("settings.modal.aria.panel")}
              >
                <SettingsView activeSection={activeSection} />
              </div>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>,
    document.body,
  );
}
