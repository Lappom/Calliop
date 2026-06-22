import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Modal } from "../ui/Modal";
import { IconButton } from "../ui/IconButton";
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

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("settings.title")}
      description={t("settings.subtitle")}
      size="full"
      hideHeader
      backdropClassName="bg-canvas/80 p-3 sm:p-4"
      panelClassName={[
        "flex h-[min(90vh,720px)] flex-col",
        "sm:h-[min(85vh,720px)]",
      ].join(" ")}
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
            <h2 className="text-display-serif m-0 text-2xl text-ink">
              {t("settings.title")}
            </h2>
            <p className="text-body-sm mt-2 text-charcoal">
              {t("settings.subtitle")}
            </p>
          </div>
          <IconButton
            label={t("settings.modal.aria.close")}
            size="md"
            className="border-hairline-strong bg-surface-card text-ink"
            onClick={onClose}
          >
            <X size={20} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          </IconButton>
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
    </Modal>
  );
}
