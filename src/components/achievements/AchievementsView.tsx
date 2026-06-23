import { useCallback, useMemo, useState } from "react";
import { MOTION_STAGGER } from "../../lib/motion/presets";
import { useAchievements } from "../../hooks/useAchievements";
import { useRefreshSpin } from "../../hooks/useRefreshSpin";
import { useUiLocale } from "../../i18n/useUiLocale";
import { Stagger } from "../motion/Stagger";
import { SnippetListToolbarButton } from "../snippets/SnippetListToolbarButton";
import { RefreshIcon } from "../ui/RefreshIcon";
import { AchievementCard } from "./AchievementCard";
import { AchievementHeroBand } from "./AchievementHeroBand";
import { AchievementLoadingSkeleton } from "./AchievementLoadingSkeleton";
import { AchievementStatusFilterBar } from "./AchievementStatusFilter";
import { AchievementTabNav } from "./AchievementTabNav";
import {
  filterAchievements,
  type AchievementCategoryFilter,
  type AchievementStatusFilter,
} from "./achievementUtils";

export function AchievementsView() {
  const { t } = useUiLocale();
  const { summary, loaded, errorMessage, reload, markSeen } = useAchievements();
  const { spinning, runRefresh } = useRefreshSpin();
  const [activeCategory, setActiveCategory] =
    useState<AchievementCategoryFilter>("all");
  const [activeStatus, setActiveStatus] =
    useState<AchievementStatusFilter>("all");

  const achievements = useMemo(
    () => summary?.achievements ?? [],
    [summary?.achievements],
  );

  const filtered = useMemo(
    () => filterAchievements(achievements, activeCategory, activeStatus),
    [achievements, activeCategory, activeStatus],
  );

  const handleSeen = useCallback(
    (id: string) => {
      void markSeen([id]);
    },
    [markSeen],
  );

  const handleRefresh = () => {
    void runRefresh(reload);
  };

  if (!loaded) {
    return <AchievementLoadingSkeleton />;
  }

  return (
    <Stagger
      className="flex flex-col gap-8"
      itemMotion="fadeUp"
      staggerDelay={MOTION_STAGGER.editorial}
    >
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-display-serif mb-2 text-4xl text-ink sm:text-5xl">
            {t("achievements.title")}
          </h1>
          <p className="text-body-sm m-0 text-charcoal">
            {t("achievements.subtitle")}
          </p>
        </div>
        <SnippetListToolbarButton
          label={t("common.refresh")}
          disabled={spinning}
          onClick={handleRefresh}
        >
          <RefreshIcon spinning={spinning} />
        </SnippetListToolbarButton>
      </header>

      {summary && <AchievementHeroBand summary={summary} />}

      {errorMessage && (
        <p className="text-body text-accent-red" role="alert">
          {errorMessage}
        </p>
      )}

      {summary && (
        <>
          <AchievementTabNav
            achievements={achievements}
            activeCategory={activeCategory}
            onChange={setActiveCategory}
          />

          <AchievementStatusFilterBar
            achievements={achievements}
            activeStatus={activeStatus}
            onChange={setActiveStatus}
          />
        </>
      )}

      {filtered.length === 0 && (
        <p className="text-body text-charcoal">{t("achievements.empty")}</p>
      )}

      <Stagger className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {filtered.map((achievement) => (
          <AchievementCard
            key={achievement.id}
            achievement={achievement}
            onSeen={handleSeen}
          />
        ))}
      </Stagger>
    </Stagger>
  );
}
