import { useUiLocale } from "../../i18n/useUiLocale";
import type { AchievementsSummary } from "../../hooks/useAchievements";
import { glowSurfaceClasses } from "../layout/glowSurface";
import { AnimatedMetricValue } from "../insight/AnimatedMetricValue";
import { ProgressBar } from "../ui/ProgressBar";
import {
  computeTierStats,
  progressPercent,
  type AchievementTierStats,
} from "./achievementUtils";

interface AchievementHeroBandProps {
  summary: AchievementsSummary;
}

export function AchievementHeroBand({ summary }: AchievementHeroBandProps) {
  const { t, formatNumber } = useUiLocale();
  const tierStats = computeTierStats(summary.achievements);
  const percent = progressPercent(summary);

  return (
    <div
      className={[
        glowSurfaceClasses("green"),
        "rounded-lg border border-hairline-strong bg-surface-card p-6 sm:p-8",
      ].join(" ")}
    >
      <div className="relative flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
        <div className="min-w-0 flex-1">
          <p className="text-caption m-0 text-charcoal">
            {t("achievements.hero.collectionLabel")}
          </p>
          <p className="text-display-serif m-0 mt-2 text-4xl text-ink sm:text-5xl">
            <AnimatedMetricValue
              value={summary.unlockedCount}
              format={(n) => formatNumber(Math.round(n))}
              className="font-[family-name:var(--font-mono)] tabular-nums"
            />
            <span className="text-charcoal"> / </span>
            <span className="font-[family-name:var(--font-mono)] tabular-nums text-charcoal">
              {formatNumber(summary.totalCount)}
            </span>
          </p>
          <p className="text-body-sm m-0 mt-2 text-charcoal">
            {t("achievements.hero.unlockedDetail", {
              percent,
            })}
          </p>
          <div className="mt-4 max-w-md">
            <ProgressBar
              value={percent}
              label={t("achievements.overallProgress")}
            />
          </div>
        </div>

        <div className="flex flex-wrap gap-3 lg:justify-end">
          <TierPill
            label={t("achievements.tiers.common")}
            stats={tierStats.common}
            accentClass="text-accent-blue"
          />
          <TierPill
            label={t("achievements.tiers.rare")}
            stats={tierStats.rare}
            accentClass="text-accent-orange"
          />
          <TierPill
            label={t("achievements.tiers.legendary")}
            stats={tierStats.legendary}
            accentClass="text-accent-green"
          />
          <TierPill
            label={t("achievements.hero.secretsLabel")}
            stats={tierStats.secrets}
            accentClass="text-accent-yellow"
          />
        </div>
      </div>
    </div>
  );
}

function TierPill({
  label,
  stats,
  accentClass,
}: {
  label: string;
  stats: AchievementTierStats["common"];
  accentClass: string;
}) {
  const { formatNumber } = useUiLocale();

  return (
    <div className="min-w-[120px] flex-1 rounded-lg border border-hairline bg-surface-elevated px-4 py-3 sm:min-w-[140px] sm:flex-none">
      <p className="text-caption m-0 text-charcoal">{label}</p>
      <p
        className={[
          "text-heading-sm m-0 mt-1 font-[family-name:var(--font-mono)] tabular-nums",
          accentClass,
        ].join(" ")}
      >
        {formatNumber(stats.unlocked)}
        <span className="text-charcoal"> / {formatNumber(stats.total)}</span>
      </p>
    </div>
  );
}
