import { useUiLocale } from "../../i18n/useUiLocale";
import type { WeekSummary } from "./insightUtils";
import { formatWeekdayLabel } from "./insightUtils";
import { InsightMetricCard } from "./InsightMetricCard";

interface InsightWeekSummaryProps {
  summary: WeekSummary;
}

export function InsightWeekSummary({ summary }: InsightWeekSummaryProps) {
  const { t, intlLocale, formatNumber } = useUiLocale();
  const bestDayLabel = summary.bestDay
    ? formatWeekdayLabel(summary.bestDay.date, intlLocale)
    : null;

  return (
    <section className="flex flex-col gap-3">
      <p className="text-caption m-0 text-charcoal">
        {t("insight.sections.weekSummary")}
      </p>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <InsightMetricCard
          label={t("insight.week.words")}
          value={formatNumber(summary.totalWords)}
          glow="blue"
        />
        <InsightMetricCard
          label={t("insight.week.dictations")}
          value={String(summary.totalDictations)}
          glow="green"
        />
        <InsightMetricCard
          label={t("insight.week.avgPerActiveDay")}
          value={
            summary.averageWordsPerDay > 0
              ? t("insight.week.avgWordsValue", {
                  count: formatNumber(summary.averageWordsPerDay),
                })
              : t("common.emDash")
          }
          glow="orange"
        />
        <InsightMetricCard
          label={t("insight.week.bestDay.label")}
          value={
            summary.bestDay && summary.bestDay.wordCount > 0
              ? t("insight.week.bestDay.value", {
                  count: formatNumber(summary.bestDay.wordCount),
                })
              : t("common.emDash")
          }
          detail={
            bestDayLabel && summary.bestDay && summary.bestDay.wordCount > 0
              ? bestDayLabel.charAt(0).toUpperCase() + bestDayLabel.slice(1)
              : t("insight.week.bestDay.empty")
          }
          glow="blue"
        />
      </div>
    </section>
  );
}
