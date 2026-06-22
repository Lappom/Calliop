import { Lock, Sparkles, Star, Trophy, Zap } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AchievementState, AchievementTier } from "../../hooks/useAchievements";
import { useUiLocale } from "../../i18n/useUiLocale";
import { glowSurfaceClasses } from "../layout/glowSurface";
import { BadgePill } from "../ui/BadgePill";
import { ProgressBar } from "../ui/ProgressBar";

const tierGlow: Record<AchievementTier, "blue" | "orange" | "green"> = {
  common: "blue",
  rare: "orange",
  legendary: "green",
};

const tierIcon: Record<AchievementTier, typeof Trophy> = {
  common: Trophy,
  rare: Star,
  legendary: Sparkles,
};

interface AchievementCardProps {
  achievement: AchievementState;
}

export function AchievementCard({ achievement }: AchievementCardProps) {
  const { t } = useTranslation();
  const { formatDate } = useUiLocale();
  const { id, tier, secret, unlocked, unlockedAt, progress } = achievement;

  const title = unlocked || !secret
    ? t(`achievements.items.${id}.title`)
    : t("achievements.lockedSecretTitle");
  const description = unlocked
    ? t(`achievements.items.${id}.description`)
    : secret
      ? t(`achievements.items.${id}.hint`)
      : t(`achievements.items.${id}.description`);
  const TierIcon = tierIcon[tier];
  const glow = tierGlow[tier];

  return (
    <article
      className={[
        unlocked ? glowSurfaceClasses(glow) : "",
        "flex h-full flex-col rounded-lg border border-hairline-strong bg-surface-card p-4",
        !unlocked ? "opacity-90" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className="flex items-start justify-between gap-3">
        <div
          className={[
            "flex size-9 shrink-0 items-center justify-center rounded-md border border-hairline-strong",
            unlocked ? "bg-surface-elevated text-ink" : "bg-surface-deep text-charcoal",
          ].join(" ")}
        >
          {unlocked ? (
            <TierIcon size={18} strokeWidth={1.5} />
          ) : secret ? (
            <Zap size={18} strokeWidth={1.5} />
          ) : (
            <Lock size={16} strokeWidth={1.5} />
          )}
        </div>
        <BadgePill active={unlocked}>
          {t(`achievements.tiers.${tier}`)}
        </BadgePill>
      </div>

      <h3
        className={[
          "text-body-md relative m-0 mt-3 font-medium text-ink",
          secret && !unlocked ? "select-none blur-[4px]" : "",
        ].join(" ")}
      >
        {title}
      </h3>

      <p className="text-caption relative m-0 mt-2 flex-1 text-charcoal">
        {description}
      </p>

      {!unlocked && progress && (
        <div className="relative mt-3">
          <ProgressBar
            value={progress.current}
            max={progress.target}
            label={t("achievements.progressLabel", {
              current: progress.current,
              target: progress.target,
            })}
          />
        </div>
      )}

      {unlocked && unlockedAt && (
        <p className="text-caption relative m-0 mt-3 text-mute">
          {t("achievements.unlockedOn", {
            date: formatDate(new Date(unlockedAt), {
              year: "numeric",
              month: "short",
              day: "numeric",
            }),
          })}
        </p>
      )}
    </article>
  );
}
