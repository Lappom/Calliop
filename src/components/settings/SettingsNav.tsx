import type { LucideIcon } from "lucide-react";
import { LayoutGroup, motion } from "motion/react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  LAYOUT_TRANSITION,
  LAYOUT_TRANSITION_REDUCED,
} from "../../lib/motion/presets";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import {
  getSettingsSections,
  settingsSectionDomId,
  type SettingsSectionId,
} from "./settingsUtils";

interface SettingsNavProps {
  activeSection: SettingsSectionId;
  onSectionChange: (section: SettingsSectionId) => void;
}

const iconProps = {
  size: 18,
  strokeWidth: 1.5,
  absoluteStrokeWidth: true,
} as const;

const SETTINGS_NAV_ACTIVE_LAYOUT_ID = "settings-nav-active";

const sidebarActiveBgClassName =
  "pointer-events-none absolute inset-0 rounded-md border border-hairline-strong bg-surface-elevated";
const sidebarActiveAccentClassName =
  "pointer-events-none absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-full bg-accent-blue";

function SidebarNavButton({
  id,
  label,
  icon: Icon,
  active,
  onSelect,
}: {
  id: SettingsSectionId;
  label: string;
  icon: LucideIcon;
  active: boolean;
  onSelect: (id: SettingsSectionId) => void;
}) {
  return (
    <button
      type="button"
      role="tab"
      id={`${settingsSectionDomId(id)}-tab`}
      aria-selected={active}
      aria-controls={`${settingsSectionDomId(id)}-panel`}
      onClick={() => onSelect(id)}
      className={[
        "group relative flex w-full items-center gap-3 rounded-md px-3 py-2 text-left",
        "text-button-sm transition-colors duration-150",
        "border border-transparent",
        active ? "text-ink" : "text-body hover:text-ink",
      ].join(" ")}
    >
      {active && (
        <>
          <span className={sidebarActiveBgClassName} aria-hidden />
          <span className={sidebarActiveAccentClassName} aria-hidden />
        </>
      )}
      <Icon
        {...iconProps}
        className={[
          "relative z-[1] shrink-0 transition-colors duration-150",
          active ? "text-accent-blue" : "text-charcoal group-hover:text-ink",
        ].join(" ")}
        aria-hidden
      />
      <span className="relative z-[1]">{label}</span>
    </button>
  );
}

function PillNavButton({
  id,
  label,
  active,
  onSelect,
  layoutTransition,
}: {
  id: SettingsSectionId;
  label: string;
  active: boolean;
  onSelect: (id: SettingsSectionId) => void;
  layoutTransition: typeof LAYOUT_TRANSITION | typeof LAYOUT_TRANSITION_REDUCED;
}) {
  return (
    <button
      type="button"
      role="tab"
      id={`${settingsSectionDomId(id)}-tab`}
      aria-selected={active}
      aria-controls={`${settingsSectionDomId(id)}-panel`}
      onClick={() => onSelect(id)}
      className={[
        "relative shrink-0 rounded-full px-3.5 py-1.5 text-button-sm transition-colors duration-150",
        active ? "text-ink" : "text-charcoal hover:text-ink",
      ].join(" ")}
    >
      {active && (
        <motion.span
          layoutId={SETTINGS_NAV_ACTIVE_LAYOUT_ID}
          className="pointer-events-none absolute inset-0 rounded-full border border-hairline-strong bg-surface-elevated"
          transition={layoutTransition}
          aria-hidden
        />
      )}
      <span className="relative whitespace-nowrap">{label}</span>
    </button>
  );
}

export function SettingsNav({
  activeSection,
  onSectionChange,
}: SettingsNavProps) {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const layoutTransition = reducedMotion
    ? LAYOUT_TRANSITION_REDUCED
    : LAYOUT_TRANSITION;
  const sections = useMemo(() => getSettingsSections(t), [t]);

  return (
    <>
      <nav
        className="calliop-scroll relative hidden w-[220px] shrink-0 flex-col gap-1 overflow-y-auto border-r border-hairline px-3 py-4 lg:flex"
        role="tablist"
        aria-orientation="vertical"
        aria-label={t("settings.modal.aria.navigation")}
      >
        <div
          className="pointer-events-none absolute inset-x-0 top-0 h-32"
          style={{
            background:
              "radial-gradient(ellipse 80% 100% at 50% 0%, var(--color-accent-blue-glow) 0%, transparent 70%)",
            opacity: 0.08,
          }}
          aria-hidden
        />
        {sections.map((section) => (
          <SidebarNavButton
            key={section.id}
            id={section.id}
            label={section.label}
            icon={section.icon}
            active={activeSection === section.id}
            onSelect={onSectionChange}
          />
        ))}
      </nav>

      <LayoutGroup id="settings-nav-pills">
        <nav
          className="calliop-scroll flex shrink-0 gap-2 overflow-x-auto border-b border-hairline px-4 py-3 lg:hidden"
          role="tablist"
          aria-orientation="horizontal"
          aria-label={t("settings.modal.aria.navigation")}
        >
          {sections.map((section) => (
            <PillNavButton
              key={section.id}
              id={section.id}
              label={section.label}
              active={activeSection === section.id}
              onSelect={onSectionChange}
              layoutTransition={layoutTransition}
            />
          ))}
        </nav>
      </LayoutGroup>
    </>
  );
}
