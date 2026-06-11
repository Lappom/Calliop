import type { WeekSummary } from "./insightUtils";
import { formatWeekdayLabel } from "./insightUtils";
import { InsightMetricCard } from "./InsightMetricCard";

interface InsightWeekSummaryProps {
  summary: WeekSummary;
}

export function InsightWeekSummary({ summary }: InsightWeekSummaryProps) {
  const bestDayLabel = summary.bestDay
    ? formatWeekdayLabel(summary.bestDay.date)
    : null;

  return (
    <section className="flex flex-col gap-3">
      <p className="text-caption m-0 text-charcoal">Synthèse — 7 derniers jours</p>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <InsightMetricCard
          label="Mots dictés"
          value={summary.totalWords.toLocaleString("fr-FR")}
          glow="blue"
        />
        <InsightMetricCard
          label="Dictées"
          value={String(summary.totalDictations)}
          glow="green"
        />
        <InsightMetricCard
          label="Moyenne / jour actif"
          value={
            summary.averageWordsPerDay > 0
              ? `${summary.averageWordsPerDay.toLocaleString("fr-FR")} mots`
              : "—"
          }
          glow="orange"
        />
        <InsightMetricCard
          label="Meilleur jour"
          value={
            summary.bestDay && summary.bestDay.wordCount > 0
              ? `${summary.bestDay.wordCount.toLocaleString("fr-FR")} mots`
              : "—"
          }
          detail={
            bestDayLabel && summary.bestDay && summary.bestDay.wordCount > 0
              ? bestDayLabel.charAt(0).toUpperCase() + bestDayLabel.slice(1)
              : "Aucune activité sur la période."
          }
          glow="blue"
        />
      </div>
    </section>
  );
}
