import type { DictationEntry } from "../../hooks/useHistory";
import { useUiLocale } from "../../i18n/useUiLocale";
import { glowSurfaceClasses } from "../layout/glowSurface";
import { computeHistoryStats } from "./historyUtils";

interface HistoryStatsBarProps {
  entries: DictationEntry[];
  totalCount?: number;
}

export function HistoryStatsBar({ entries, totalCount }: HistoryStatsBarProps) {
  const { t, formatNumber } = useUiLocale();
  const { count, totalWords, avgLatency } = computeHistoryStats(entries);
  const displayCount = totalCount ?? count;

  if (displayCount === 0) {
    return null;
  }

  return (
    <div className="grid gap-3 sm:grid-cols-3">
      <StatCard label={t("history.stats.dictations")} value={String(displayCount)} />
      <StatCard
        label={t("history.stats.words")}
        value={formatNumber(totalWords)}
      />
      <StatCard
        label={t("history.stats.avgLatency")}
        value={avgLatency > 0 ? `${avgLatency} ms` : t("common.emDash")}
      />
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div
      className={[
        glowSurfaceClasses("blue"),
        "rounded-lg border border-hairline-strong bg-surface-card px-4 py-3",
      ].join(" ")}
    >
      <p className="text-caption relative m-0 text-charcoal">{label}</p>
      <p className="text-heading-sm relative m-0 mt-1 text-ink">{value}</p>
    </div>
  );
}
