import type {
  AchievementCategory,
  AchievementState,
  AchievementTier,
  AchievementsSummary,
} from "../../hooks/useAchievements";

export type AchievementStatusFilter = "all" | "unlocked" | "locked" | "new";

export type AchievementCategoryFilter = AchievementCategory | "all";

const TIER_RANK: Record<AchievementTier, number> = {
  legendary: 0,
  rare: 1,
  common: 2,
};

export interface AchievementTierStats {
  common: { unlocked: number; total: number };
  rare: { unlocked: number; total: number };
  legendary: { unlocked: number; total: number };
  secrets: { unlocked: number; total: number };
}

export function computeTierStats(
  achievements: AchievementState[],
): AchievementTierStats {
  const stats: AchievementTierStats = {
    common: { unlocked: 0, total: 0 },
    rare: { unlocked: 0, total: 0 },
    legendary: { unlocked: 0, total: 0 },
    secrets: { unlocked: 0, total: 0 },
  };

  for (const achievement of achievements) {
    stats[achievement.tier].total += 1;
    if (achievement.unlocked) {
      stats[achievement.tier].unlocked += 1;
    }
    if (achievement.secret) {
      stats.secrets.total += 1;
      if (achievement.unlocked) {
        stats.secrets.unlocked += 1;
      }
    }
  }

  return stats;
}

export function countByCategory(
  achievements: AchievementState[],
  category: AchievementCategoryFilter,
): { unlocked: number; total: number } {
  const filtered =
    category === "all"
      ? achievements
      : achievements.filter((a) => a.category === category);

  return {
    unlocked: filtered.filter((a) => a.unlocked).length,
    total: filtered.length,
  };
}

export function countByStatus(
  achievements: AchievementState[],
  status: AchievementStatusFilter,
): number {
  switch (status) {
    case "unlocked":
      return achievements.filter((a) => a.unlocked).length;
    case "locked":
      return achievements.filter((a) => !a.unlocked).length;
    case "new":
      return achievements.filter((a) => a.unlocked && !a.seen).length;
    default:
      return achievements.length;
  }
}

export function filterAchievements(
  achievements: AchievementState[],
  category: AchievementCategoryFilter,
  status: AchievementStatusFilter,
): AchievementState[] {
  let result = achievements;

  if (category !== "all") {
    result = result.filter((a) => a.category === category);
  }

  switch (status) {
    case "unlocked":
      result = result.filter((a) => a.unlocked);
      break;
    case "locked":
      result = result.filter((a) => !a.unlocked);
      break;
    case "new":
      result = result.filter((a) => a.unlocked && !a.seen);
      break;
    default:
      break;
  }

  return sortAchievements(result);
}

export function sortAchievements(
  achievements: AchievementState[],
): AchievementState[] {
  return [...achievements].sort((a, b) => {
    const aNew = a.unlocked && !a.seen ? 0 : 1;
    const bNew = b.unlocked && !b.seen ? 0 : 1;
    if (aNew !== bNew) {
      return aNew - bNew;
    }

    const tierDiff = TIER_RANK[a.tier] - TIER_RANK[b.tier];
    if (tierDiff !== 0) {
      return tierDiff;
    }

    const aUnlocked = a.unlocked ? 0 : 1;
    const bUnlocked = b.unlocked ? 0 : 1;
    if (aUnlocked !== bUnlocked) {
      return aUnlocked - bUnlocked;
    }

    return a.id.localeCompare(b.id);
  });
}

export function progressPercent(summary: AchievementsSummary): number {
  if (summary.totalCount === 0) {
    return 0;
  }
  return Math.round((summary.unlockedCount / summary.totalCount) * 100);
}
