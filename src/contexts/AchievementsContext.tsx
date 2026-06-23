import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

export type AchievementTier = "common" | "rare" | "legendary";

export type AchievementCategory =
  | "milestones"
  | "streaks"
  | "speed"
  | "explorer"
  | "learner"
  | "secrets";

export interface AchievementProgress {
  current: number;
  target: number;
}

export interface AchievementState {
  id: string;
  tier: AchievementTier;
  category: AchievementCategory;
  secret: boolean;
  unlocked: boolean;
  unlockedAt?: string | null;
  seen: boolean;
  progress?: AchievementProgress | null;
}

export interface AchievementsSummary {
  achievements: AchievementState[];
  unlockedCount: number;
  totalCount: number;
  unseenCount: number;
}

export interface AchievementUnlockedPayload {
  id: string;
  tier: AchievementTier;
  secret: boolean;
}

interface AchievementsContextValue {
  summary: AchievementsSummary | null;
  loaded: boolean;
  errorMessage: string | null;
  reload: () => Promise<void>;
  markSeen: (ids?: string[]) => Promise<void>;
}

const AchievementsContext = createContext<AchievementsContextValue | null>(null);

export function AchievementsProvider({ children }: { children: ReactNode }) {
  const [summary, setSummary] = useState<AchievementsSummary | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      const data = await invoke<AchievementsSummary>("get_achievements");
      setSummary(data);
      setErrorMessage(null);
    } catch (error) {
      setErrorMessage(String(error));
    } finally {
      setLoaded(true);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    const unlistenHistory = listen("history-updated", () => {
      void reload();
    });
    const unlistenAchievement = listen("achievement-unlocked", () => {
      void reload();
    });
    return () => {
      void unlistenHistory.then((drop) => drop());
      void unlistenAchievement.then((drop) => drop());
    };
  }, [reload]);

  const markSeen = useCallback(
    async (ids?: string[]) => {
      await invoke("mark_achievements_seen", { ids: ids ?? null });
      void reload();
    },
    [reload],
  );

  const value = useMemo(
    () => ({ summary, loaded, errorMessage, reload, markSeen }),
    [summary, loaded, errorMessage, reload, markSeen],
  );

  return (
    <AchievementsContext.Provider value={value}>
      {children}
    </AchievementsContext.Provider>
  );
}

export function useAchievementsContext(): AchievementsContextValue {
  const context = useContext(AchievementsContext);
  if (!context) {
    throw new Error(
      "useAchievementsContext must be used within AchievementsProvider",
    );
  }
  return context;
}
