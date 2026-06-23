import { LayoutGroup, motion } from "motion/react";
import { useCallback, useMemo, type KeyboardEvent } from "react";
import {
  LAYOUT_TRANSITION,
  LAYOUT_TRANSITION_REDUCED,
} from "../../lib/motion/presets";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import { useUiLocale } from "../../i18n/useUiLocale";
import type { AchievementState } from "../../hooks/useAchievements";
import {
  countByCategory,
  type AchievementCategoryFilter,
} from "./achievementUtils";

const CATEGORY_IDS: AchievementCategoryFilter[] = [
  "all",
  "milestones",
  "streaks",
  "speed",
  "explorer",
  "learner",
  "secrets",
];

const ACTIVE_TAB_LAYOUT_ID = "achievement-tab-active";

interface AchievementTabNavProps {
  achievements: AchievementState[];
  activeCategory: AchievementCategoryFilter;
  onChange: (category: AchievementCategoryFilter) => void;
}

export function AchievementTabNav({
  achievements,
  activeCategory,
  onChange,
}: AchievementTabNavProps) {
  const { t } = useUiLocale();
  const reducedMotion = useReducedMotion();
  const layoutTransition = reducedMotion
    ? LAYOUT_TRANSITION_REDUCED
    : LAYOUT_TRANSITION;

  const tabs = useMemo(
    () =>
      CATEGORY_IDS.map((id) => {
        const counts = countByCategory(achievements, id);
        return {
          id,
          label: t(`achievements.categories.${id}`),
          counts,
        };
      }),
    [achievements, t],
  );

  const handleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDivElement>) => {
      const index = CATEGORY_IDS.indexOf(activeCategory);
      if (index < 0) {
        return;
      }

      let nextIndex: number | null = null;
      if (event.key === "ArrowRight") {
        nextIndex = (index + 1) % CATEGORY_IDS.length;
      } else if (event.key === "ArrowLeft") {
        nextIndex = (index - 1 + CATEGORY_IDS.length) % CATEGORY_IDS.length;
      } else if (event.key === "Home") {
        nextIndex = 0;
      } else if (event.key === "End") {
        nextIndex = CATEGORY_IDS.length - 1;
      }

      if (nextIndex == null) {
        return;
      }

      event.preventDefault();
      const nextCategory = CATEGORY_IDS[nextIndex];
      onChange(nextCategory);
      document.getElementById(`achievement-tab-${nextCategory}`)?.focus();
    },
    [activeCategory, onChange],
  );

  return (
    <LayoutGroup id="achievement-tabs">
      <div
        role="tablist"
        aria-label={t("achievements.tabs.aria")}
        className="flex flex-wrap gap-2"
        onKeyDown={handleKeyDown}
      >
        {tabs.map((tab) => {
          const selected = activeCategory === tab.id;
          return (
            <button
              key={tab.id}
              id={`achievement-tab-${tab.id}`}
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
              <span className="relative inline-flex items-center gap-1.5">
                {tab.label}
                <span className="font-[family-name:var(--font-mono)] text-caption tabular-nums text-ash">
                  {tab.counts.unlocked}/{tab.counts.total}
                </span>
              </span>
            </button>
          );
        })}
      </div>
    </LayoutGroup>
  );
}
