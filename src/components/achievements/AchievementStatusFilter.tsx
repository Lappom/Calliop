import { useMemo } from "react";
import { useUiLocale } from "../../i18n/useUiLocale";
import type { AchievementState } from "../../hooks/useAchievements";
import {
  countByStatus,
  type AchievementStatusFilter,
} from "./achievementUtils";

const STATUS_IDS: AchievementStatusFilter[] = [
  "all",
  "unlocked",
  "locked",
  "new",
];

interface AchievementStatusFilterProps {
  achievements: AchievementState[];
  activeStatus: AchievementStatusFilter;
  onChange: (status: AchievementStatusFilter) => void;
}

export function AchievementStatusFilterBar({
  achievements,
  activeStatus,
  onChange,
}: AchievementStatusFilterProps) {
  const { t } = useUiLocale();

  const options = useMemo(
    () =>
      STATUS_IDS.map((id) => ({
        id,
        label: t(`achievements.status.${id}`),
        count: countByStatus(achievements, id),
      })),
    [achievements, t],
  );

  return (
    <div
      role="group"
      aria-label={t("achievements.status.aria")}
      className="flex flex-wrap gap-2"
    >
      {options.map((option) => {
        const selected = activeStatus === option.id;
        return (
          <button
            key={option.id}
            type="button"
            aria-pressed={selected}
            onClick={() => onChange(option.id)}
            className={[
              "rounded-full border px-3 py-1.5 text-caption transition-colors duration-150 ease-out active:scale-[0.97]",
              selected
                ? "border-hairline-strong bg-surface-elevated text-ink"
                : "border-hairline bg-transparent text-charcoal hover:text-ink",
            ].join(" ")}
          >
            <span className="inline-flex items-center gap-1.5">
              {option.label}
              <span className="font-[family-name:var(--font-mono)] tabular-nums text-ash">
                {option.count}
              </span>
            </span>
          </button>
        );
      })}
    </div>
  );
}
