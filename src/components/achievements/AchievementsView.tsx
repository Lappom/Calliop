import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AchievementCategory } from "../../hooks/useAchievements";
import { useAchievements } from "../../hooks/useAchievements";
import { useRefreshSpin } from "../../hooks/useRefreshSpin";
import { SectionGlow } from "../layout/SectionGlow";
import { Stagger } from "../motion/Stagger";
import { RefreshIcon } from "../ui/RefreshIcon";
import { ProgressBar } from "../ui/ProgressBar";
import { AchievementCard } from "./AchievementCard";

const CATEGORY_IDS: (AchievementCategory | "all")[] = [
  "all",
  "milestones",
  "streaks",
  "speed",
  "explorer",
  "learner",
  "secrets",
];

export function AchievementsView() {
  const { t } = useTranslation();
  const { summary, loaded, errorMessage, reload, markSeen } = useAchievements();
  const { spinning, runRefresh } = useRefreshSpin();
  const [activeCategory, setActiveCategory] = useState<AchievementCategory | "all">(
    "all",
  );

  const filtered = useMemo(() => {
    if (!summary) {
      return [];
    }
    if (activeCategory === "all") {
      return summary.achievements;
    }
    return summary.achievements.filter((a) => a.category === activeCategory);
  }, [summary, activeCategory]);

  const progressPercent = summary
    ? Math.round((summary.unlockedCount / summary.totalCount) * 100)
    : 0;

  useEffect(() => {
    if (summary && summary.unseenCount > 0) {
      void markSeen();
    }
  }, [summary, markSeen]);

  return (
    <div className="flex flex-1 flex-col gap-6 p-4 sm:p-6">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-heading-md m-0 font-medium text-ink">
            {t("achievements.title")}
          </h1>
          <p className="text-body-sm m-0 mt-1 text-charcoal">
            {t("achievements.subtitle")}
          </p>
        </div>
        <button
          type="button"
          onClick={() => runRefresh(reload)}
          className="inline-flex items-center gap-2 rounded-md border border-hairline-strong bg-surface-elevated px-3 py-2 text-body-sm text-ink transition-transform duration-150 ease-out active:scale-[0.97]"
        >
          <RefreshIcon spinning={spinning} />
          {t("common.refresh")}
        </button>
      </header>

      {summary && (
        <SectionGlow glow="green" className="rounded-lg border border-hairline-strong p-4 sm:p-5">
          <div className="relative flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <p className="text-caption m-0 text-charcoal">
                {t("achievements.collectionProgress")}
              </p>
              <p className="text-heading-sm m-0 mt-1 font-medium text-ink">
                {t("achievements.unlockedCount", {
                  count: summary.unlockedCount,
                  total: summary.totalCount,
                })}
              </p>
            </div>
            <div className="w-full sm:max-w-xs">
              <ProgressBar value={progressPercent} label={t("achievements.overallProgress")} />
            </div>
          </div>
        </SectionGlow>
      )}

      <div className="flex flex-wrap gap-2">
        {CATEGORY_IDS.map((category) => (
          <button
            key={category}
            type="button"
            onClick={() => setActiveCategory(category)}
            className={[
              "rounded-full border px-3 py-1.5 text-caption transition-colors duration-150 ease-out active:scale-[0.97]",
              activeCategory === category
                ? "border-hairline-strong bg-surface-elevated text-ink"
                : "border-hairline bg-transparent text-charcoal hover:text-ink",
            ].join(" ")}
          >
            {t(`achievements.categories.${category}`)}
          </button>
        ))}
      </div>

      {!loaded && (
        <p className="text-body text-charcoal">{t("common.loading")}</p>
      )}

      {errorMessage && (
        <p className="text-body text-accent-red" role="alert">
          {errorMessage}
        </p>
      )}

      {loaded && filtered.length === 0 && (
        <p className="text-body text-charcoal">{t("achievements.empty")}</p>
      )}

      <Stagger className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {filtered.map((achievement) => (
          <AchievementCard key={achievement.id} achievement={achievement} />
        ))}
      </Stagger>
    </div>
  );
}
