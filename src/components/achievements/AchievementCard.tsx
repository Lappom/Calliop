import { Lock, Sparkles, Star, Trophy, Zap } from "lucide-react";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { AchievementState, AchievementTier } from "../../hooks/useAchievements";
import { useUiLocale } from "../../i18n/useUiLocale";
import { glowSurfaceClasses } from "../layout/glowSurface";
import { BadgePill } from "../ui/BadgePill";
import { ProgressBar } from "../ui/ProgressBar";
import { tierUnlockedBorderClass } from "./achievementTierStyles";

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
  onSeen?: (id: string) => void;
}

export function AchievementCard({ achievement, onSeen }: AchievementCardProps) {
  const { t } = useTranslation();
  const { formatDate } = useUiLocale();
  const cardRef = useRef<HTMLElement>(null);
  const { id, tier, secret, unlocked, unlockedAt, seen, progress } =
    achievement;

  const isNew = unlocked && !seen;
  const isSecretLocked = secret && !unlocked;

  const title = unlocked || !secret
    ? t(`achievements.items.${id}.title`)
    : t("achievements.lockedSecretTitle");
  const hint = secret && !unlocked
    ? t(`achievements.items.${id}.hint`)
    : null;
  const description = unlocked
    ? t(`achievements.items.${id}.description`)
    : secret
      ? hint
      : t(`achievements.items.${id}.description`);

  const TierIcon = tierIcon[tier];
  const glow = tierGlow[tier];

  useEffect(() => {
    if (!isNew || !onSeen) {
      return;
    }

    const element = cardRef.current;
    if (!element) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        const entry = entries[0];
        if (entry?.isIntersecting) {
          onSeen(id);
          observer.disconnect();
        }
      },
      { threshold: 0.5 },
    );

    observer.observe(element);
    return () => observer.disconnect();
  }, [id, isNew, onSeen]);

  return (
    <article
      ref={cardRef}
      data-achievement-id={id}
      aria-label={
        isSecretLocked && hint
          ? `${t("achievements.lockedSecretTitle")}: ${hint}`
          : undefined
      }
      className={[
        unlocked
          ? glowSurfaceClasses(glow, isNew ? "normal" : undefined)
          : "",
        "relative flex h-full flex-col rounded-lg border p-4",
        unlocked
          ? `${tierUnlockedBorderClass[tier]} bg-surface-card`
          : isSecretLocked
            ? "border-accent-orange/30 bg-surface-deep"
            : "border-hairline-strong bg-surface-deep opacity-90",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {isNew && (
        <span className="absolute right-3 top-3 inline-flex items-center gap-1.5 rounded-full border border-accent-orange/40 bg-surface-elevated px-2 py-0.5 text-caption text-accent-orange">
          <span
            className="size-1.5 rounded-full bg-accent-orange"
            aria-hidden
          />
          {t("achievements.badge.new")}
        </span>
      )}

      <div className="flex items-start justify-between gap-3">
        <div
          className={[
            "flex size-9 shrink-0 items-center justify-center rounded-md border border-hairline-strong",
            unlocked ? "bg-surface-elevated text-ink" : "bg-surface-card text-charcoal",
          ].join(" ")}
        >
          {unlocked ? (
            <TierIcon size={18} strokeWidth={1.5} />
          ) : secret ? (
            <Zap size={18} strokeWidth={1.5} className="text-accent-orange" />
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
          "text-body-md m-0 mt-3 font-medium text-ink",
          isSecretLocked ? "select-none blur-[4px]" : "",
        ].join(" ")}
        aria-hidden={isSecretLocked ? true : undefined}
      >
        {title}
      </h3>

      <p
        className={[
          "text-caption m-0 mt-2 flex-1 text-charcoal",
          isSecretLocked ? "italic" : "",
        ].join(" ")}
      >
        {description}
      </p>

      {!unlocked && progress && (
        <div className="mt-3">
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
        <p className="text-caption m-0 mt-3 text-mute">
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
