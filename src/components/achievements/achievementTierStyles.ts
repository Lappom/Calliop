import type { AchievementTier } from "../../hooks/useAchievements";

export const tierUnlockedBorderClass: Record<AchievementTier, string> = {
  common: "border-hairline-strong",
  rare: "border-accent-orange/60 shadow-[0_0_24px_rgba(255,128,31,0.18)]",
  legendary:
    "!border-accent-green/60 shadow-[0_0_28px_rgba(17,255,153,0.2)]",
};
