import { Flame } from "lucide-react";
import { useUiLocale } from "../../i18n/useUiLocale";
import type { StreakInfo } from "../../hooks/useInsights";
import { glowSurfaceClasses } from "../layout/glowSurface";
import { AnimatedMetricValue } from "./AnimatedMetricValue";

interface InsightStreakCardProps {
  streak: StreakInfo;
}

export function InsightStreakCard({ streak }: InsightStreakCardProps) {
  const { t } = useUiLocale();
  const { currentStreak, bestStreak, activeToday } = streak;
  const hasStreak = currentStreak > 0 || bestStreak > 0;

  return (
    <div
      className={[
        glowSurfaceClasses("orange"),
        "insight-metric-hover flex h-full flex-col justify-between rounded-lg border border-hairline-strong bg-surface-card p-5 sm:p-6",
      ].join(" ")}
    >
      <div className="relative flex items-start justify-between gap-3">
        <div>
          <p className="text-caption m-0 text-charcoal">
            {t("insight.streak.label")}
          </p>
          <p className="text-display-serif m-0 mt-2 text-3xl text-ink sm:text-4xl">
            {hasStreak ? (
              <AnimatedMetricValue value={currentStreak} />
            ) : (
              t("common.emDash")
            )}
          </p>
          <p className="text-body-sm m-0 mt-2 text-charcoal">
            {hasStreak
              ? t("insight.streak.currentDetail", { count: currentStreak })
              : t("insight.streak.empty")}
          </p>
        </div>
        <span
          className={[
            "inline-flex size-10 shrink-0 items-center justify-center rounded-lg border border-hairline-strong bg-surface-elevated",
            activeToday && currentStreak > 0
              ? "text-accent-orange"
              : "text-charcoal",
          ].join(" ")}
          aria-hidden
        >
          <Flame size={18} strokeWidth={1.75} />
        </span>
      </div>
      <p className="text-caption relative m-0 mt-4 text-ash">
        {bestStreak > 0
          ? t("insight.streak.best", { count: bestStreak })
          : t("insight.streak.bestEmpty")}
      </p>
    </div>
  );
}
