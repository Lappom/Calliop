import { useUiLocale } from "../../i18n/useUiLocale";
import type { WeekSummary } from "./insightUtils";
import { formatWeekdayLabel } from "./insightUtils";
import { AnimatedMetricValue } from "./AnimatedMetricValue";

interface InsightMetricStripProps {
  summary: WeekSummary;
}

interface StripItem {
  label: string;
  value: string;
  numericValue?: number;
  detail?: string;
}

export function InsightMetricStrip({ summary }: InsightMetricStripProps) {
  const { t, intlLocale, formatNumber } = useUiLocale();
  const bestDayLabel = summary.bestDay
    ? formatWeekdayLabel(summary.bestDay.date, intlLocale)
    : null;

  const items: StripItem[] = [
    {
      label: t("insight.week.words"),
      value: formatNumber(summary.totalWords),
      numericValue: summary.totalWords,
    },
    {
      label: t("insight.week.dictations"),
      value: String(summary.totalDictations),
      numericValue: summary.totalDictations,
    },
    {
      label: t("insight.week.avgPerActiveDay"),
      value:
        summary.averageWordsPerDay > 0
          ? t("insight.week.avgWordsValue", {
              count: formatNumber(summary.averageWordsPerDay),
            })
          : t("common.emDash"),
      numericValue:
        summary.averageWordsPerDay > 0 ? summary.averageWordsPerDay : undefined,
    },
    {
      label: t("insight.week.bestDay.label"),
      value:
        summary.bestDay && summary.bestDay.wordCount > 0
          ? t("insight.week.bestDay.value", {
              count: formatNumber(summary.bestDay.wordCount),
            })
          : t("common.emDash"),
      numericValue:
        summary.bestDay && summary.bestDay.wordCount > 0
          ? summary.bestDay.wordCount
          : undefined,
      detail:
        bestDayLabel && summary.bestDay && summary.bestDay.wordCount > 0
          ? bestDayLabel.charAt(0).toUpperCase() + bestDayLabel.slice(1)
          : t("insight.week.bestDay.empty"),
    },
  ];

  return (
    <div className="grid gap-px overflow-hidden rounded-lg border border-hairline-strong bg-hairline sm:grid-cols-2 lg:grid-cols-4">
      {items.map((item, index) => (
        <div
          key={item.label}
          className={[
            "bg-surface-card px-4 py-3 sm:px-5 sm:py-4",
            index > 0 ? "border-t border-hairline sm:border-t-0 sm:border-l" : "",
          ].join(" ")}
        >
          <p className="text-caption m-0 text-charcoal">{item.label}</p>
          <p className="text-heading-sm m-0 mt-1 font-[family-name:var(--font-mono)] tabular-nums text-ink">
            {item.numericValue != null ? (
              <AnimatedMetricValue
                value={item.numericValue}
                format={(n) =>
                  item.label === t("insight.week.avgPerActiveDay")
                    ? t("insight.week.avgWordsValue", {
                        count: formatNumber(Math.round(n)),
                      })
                    : item.label === t("insight.week.bestDay.label")
                      ? t("insight.week.bestDay.value", {
                          count: formatNumber(Math.round(n)),
                        })
                      : formatNumber(Math.round(n))
                }
              />
            ) : (
              item.value
            )}
          </p>
          {item.detail && (
            <p className="text-caption m-0 mt-1 text-ash">{item.detail}</p>
          )}
        </div>
      ))}
    </div>
  );
}
