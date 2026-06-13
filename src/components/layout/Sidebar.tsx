import { LayoutGroup, motion } from "motion/react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  LAYOUT_TRANSITION,
  LAYOUT_TRANSITION_REDUCED,
} from "../../lib/motion/presets";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import type { AppView } from "../../lib/views";
import {
  getBottomNavItems,
  getNavSections,
  type BottomNavItem,
  type NavItem,
} from "./navItems";

interface SidebarProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  settingsOpen: boolean;
  onOpenSettings: () => void;
  open: boolean;
  onClose: () => void;
}

const iconProps = {
  size: 18,
  strokeWidth: 1.5,
  absoluteStrokeWidth: true,
} as const;

const SIDEBAR_ACTIVE_BG_ID = "sidebar-nav-active-bg";
const SIDEBAR_ACTIVE_ACCENT_ID = "sidebar-nav-accent";

const activeBgClassName =
  "pointer-events-none absolute inset-0 rounded-md border border-hairline-strong bg-surface-elevated";
const activeAccentClassName =
  "pointer-events-none absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-full bg-accent-blue";

function ActiveIndicator({
  animated,
  layoutTransition,
}: {
  animated: boolean;
  layoutTransition: typeof LAYOUT_TRANSITION | typeof LAYOUT_TRANSITION_REDUCED;
}) {
  if (animated) {
    return (
      <>
        <motion.span
          layoutId={SIDEBAR_ACTIVE_BG_ID}
          className={activeBgClassName}
          transition={layoutTransition}
          aria-hidden
        />
        <motion.span
          layoutId={SIDEBAR_ACTIVE_ACCENT_ID}
          className={activeAccentClassName}
          transition={layoutTransition}
          aria-hidden
        />
      </>
    );
  }

  return (
    <>
      <span className={activeBgClassName} aria-hidden />
      <span className={activeAccentClassName} aria-hidden />
    </>
  );
}

function NavButton({
  item,
  active,
  onNavigate,
  onClose,
  layoutTransition,
  animateIndicator,
}: {
  item: NavItem;
  active: boolean;
  onNavigate: (view: AppView) => void;
  onClose: () => void;
  layoutTransition: typeof LAYOUT_TRANSITION | typeof LAYOUT_TRANSITION_REDUCED;
  animateIndicator: boolean;
}) {
  const Icon = item.icon;

  return (
    <button
      type="button"
      onClick={() => {
        onNavigate(item.id);
        onClose();
      }}
      className={[
        "group relative flex w-full items-center gap-3 rounded-md px-3 py-2",
        "font-[family-name:var(--font-body)] text-sm font-medium tracking-wide",
        "border border-transparent transition-colors duration-150",
        active ? "text-ink" : "text-body hover:text-ink",
      ].join(" ")}
      aria-current={active ? "page" : undefined}
    >
      {active && (
        <ActiveIndicator
          animated={animateIndicator}
          layoutTransition={layoutTransition}
        />
      )}
      <Icon
        {...iconProps}
        className={[
          "relative z-[1] shrink-0 transition-colors duration-150",
          active ? "text-accent-blue" : "text-charcoal group-hover:text-ink",
        ].join(" ")}
        aria-hidden
      />
      <span className="relative z-[1]">{item.label}</span>
    </button>
  );
}

function SettingsNavButton({
  item,
  active,
  onOpenSettings,
  onClose,
}: {
  item: BottomNavItem;
  active: boolean;
  onOpenSettings: () => void;
  onClose: () => void;
}) {
  const Icon = item.icon;

  return (
    <button
      type="button"
      onClick={() => {
        onOpenSettings();
        onClose();
      }}
      className={[
        "group relative flex w-full items-center gap-3 rounded-md px-3 py-2",
        "font-[family-name:var(--font-body)] text-sm font-medium tracking-wide",
        "border border-transparent transition-colors duration-150",
        active ? "text-ink" : "text-body hover:text-ink",
      ].join(" ")}
      aria-expanded={active}
    >
      {active && (
        <>
          <span className={activeBgClassName} aria-hidden />
          <span className={activeAccentClassName} aria-hidden />
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
      <span className="relative z-[1]">{item.label}</span>
    </button>
  );
}

export function Sidebar({
  currentView,
  onNavigate,
  settingsOpen,
  onOpenSettings,
  open,
  onClose,
}: SidebarProps) {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const layoutTransition = reducedMotion
    ? LAYOUT_TRANSITION_REDUCED
    : LAYOUT_TRANSITION;
  const navSections = useMemo(() => getNavSections(t), [t]);
  const bottomNavItems = useMemo(() => getBottomNavItems(t), [t]);

  const navButtonProps = {
    onNavigate,
    onClose,
    layoutTransition,
  };

  return (
    <>
      {open && (
        <button
          type="button"
          className="fixed inset-x-0 bottom-0 top-8 z-30 bg-black/60 lg:hidden"
          aria-label={t("nav.aria.closeMenu")}
          onClick={onClose}
        />
      )}

      <aside
        className={[
          "fixed bottom-0 left-0 top-8 z-40 flex w-[220px] min-w-[220px] max-w-[220px] shrink-0 flex-col",
          "border-r border-hairline bg-canvas transition-transform duration-200 ease-out",
          "lg:static lg:h-full lg:min-h-0 lg:translate-x-0",
          open ? "translate-x-0" : "-translate-x-full lg:translate-x-0",
        ].join(" ")}
        aria-label={t("nav.aria.navigation")}
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

        <div className="relative hidden px-5 pb-4 pt-6 lg:block">
          <button
            type="button"
            onClick={() => onNavigate("main")}
            className="text-display-serif text-2xl text-ink transition-opacity hover:opacity-80"
          >
            {t("nav.brand")}
          </button>
        </div>

        <LayoutGroup id="sidebar-nav">
          <nav
            className="calliop-scroll relative flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto px-3 pt-6 lg:pt-0"
            aria-label={t("nav.aria.mainNavigation")}
          >
            {navSections.map((section, sectionIndex) => (
              <div
                key={section.label ?? section.items[0]?.id ?? sectionIndex}
                className={
                  sectionIndex > 0 ? "mt-4 border-t border-hairline pt-4" : undefined
                }
                role={section.label ? "group" : undefined}
                aria-label={section.label}
              >
                {section.label && (
                  <p className="mb-1.5 px-3 text-[10px] font-medium uppercase tracking-[0.14em] text-ash">
                    {section.label}
                  </p>
                )}
                <div className="flex flex-col gap-1">
                  {section.items.map((item) => (
                    <NavButton
                      key={item.id}
                      item={item}
                      active={currentView === item.id}
                      animateIndicator
                      {...navButtonProps}
                    />
                  ))}
                </div>
              </div>
            ))}
          </nav>

          <div className="relative shrink-0 flex flex-col gap-1 border-t border-hairline px-3 py-4">
            {bottomNavItems.map((item) => (
              <SettingsNavButton
                key={item.id}
                item={item}
                active={settingsOpen}
                onOpenSettings={onOpenSettings}
                onClose={onClose}
              />
            ))}
          </div>
        </LayoutGroup>
      </aside>
    </>
  );
}
