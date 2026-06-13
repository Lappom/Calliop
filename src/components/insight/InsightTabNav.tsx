import { LayoutGroup, motion } from "motion/react";
import { useCallback, useMemo, type KeyboardEvent } from "react";
import {
  LAYOUT_TRANSITION,
  LAYOUT_TRANSITION_REDUCED,
} from "../../lib/motion/presets";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import { useUiLocale } from "../../i18n/useUiLocale";
import {
  INSIGHT_TAB_IDS,
  type InsightTabId,
} from "./insightTabs";

const ACTIVE_TAB_LAYOUT_ID = "insight-tab-active";

interface InsightTabNavProps {
  activeTab: InsightTabId;
  onChange: (tab: InsightTabId) => void;
}

export function InsightTabNav({ activeTab, onChange }: InsightTabNavProps) {
  const { t } = useUiLocale();
  const reducedMotion = useReducedMotion();
  const layoutTransition = reducedMotion
    ? LAYOUT_TRANSITION_REDUCED
    : LAYOUT_TRANSITION;

  const tabs = useMemo(
    () =>
      INSIGHT_TAB_IDS.map((id) => ({
        id,
        label: t(`insight.tabs.${id}`),
      })),
    [t],
  );

  const handleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDivElement>) => {
      const index = INSIGHT_TAB_IDS.indexOf(activeTab);
      if (index < 0) {
        return;
      }

      let nextIndex: number | null = null;
      if (event.key === "ArrowRight") {
        nextIndex = (index + 1) % INSIGHT_TAB_IDS.length;
      } else if (event.key === "ArrowLeft") {
        nextIndex =
          (index - 1 + INSIGHT_TAB_IDS.length) % INSIGHT_TAB_IDS.length;
      } else if (event.key === "Home") {
        nextIndex = 0;
      } else if (event.key === "End") {
        nextIndex = INSIGHT_TAB_IDS.length - 1;
      }

      if (nextIndex == null) {
        return;
      }

      event.preventDefault();
      const nextTab = INSIGHT_TAB_IDS[nextIndex];
      onChange(nextTab);
      document.getElementById(`insight-tab-${nextTab}`)?.focus();
    },
    [activeTab, onChange],
  );

  return (
    <LayoutGroup id="insight-tabs">
      <div
        role="tablist"
        aria-label={t("insight.tabs.aria")}
        className="flex flex-wrap gap-2"
        onKeyDown={handleKeyDown}
      >
        {tabs.map((tab) => {
          const selected = activeTab === tab.id;
          return (
            <button
              key={tab.id}
              id={`insight-tab-${tab.id}`}
              type="button"
              role="tab"
              aria-selected={selected}
              tabIndex={selected ? 0 : -1}
              onClick={() => onChange(tab.id)}
              className={[
                "relative rounded-full px-3.5 py-1.5 text-button-sm transition-colors",
                selected ? "text-ink" : "text-charcoal hover:text-ink",
              ].join(" ")}
            >
              {selected && (
                <motion.span
                  layoutId={ACTIVE_TAB_LAYOUT_ID}
                  className="pointer-events-none absolute inset-0 rounded-full border border-hairline-strong bg-surface-elevated"
                  transition={layoutTransition}
                  aria-hidden
                />
              )}
              <span className="relative">{tab.label}</span>
            </button>
          );
        })}
      </div>
    </LayoutGroup>
  );
}
