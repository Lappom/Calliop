import { Clock } from "lucide-react";
import { useUiLocale } from "../../i18n/useUiLocale";
import type { EstimatedTimeSaved } from "../../hooks/useInsights";
import { glowSurfaceClasses } from "../layout/glowSurface";
import { AnimatedMetricValue } from "./AnimatedMetricValue";
import { formatTimeSaved } from "./insightUtils";

interface InsightTimeSavedCardProps {
  timeSaved: EstimatedTimeSaved;
}

export function InsightTimeSavedCard({ timeSaved }: InsightTimeSavedCardProps) {
  const { t } = useUiLocale();
  const { minutesSaved, baselineWpm } = timeSaved;
  const hasSavings = minutesSaved > 0;

  return (
    <div
      className={[
        glowSurfaceClasses("green"),
        "insight-metric-hover flex h-full flex-col justify-between rounded-lg border border-hairline-strong bg-surface-card p-5 sm:p-6",
      ].join(" ")}
    >
      <div className="relative flex items-start justify-between gap-3">
        <div>
          <p className="text-caption m-0 text-charcoal">
            {t("insight.timeSaved.label")}
          </p>
          <p className="text-display-serif m-0 mt-2 text-3xl text-ink sm:text-4xl">
            {hasSavings ? (
              <AnimatedMetricValue
                value={minutesSaved}
                format={(n) => formatTimeSaved(Math.round(n), t)}
              />
            ) : (
              t("common.emDash")
            )}
          </p>
          <p className="text-body-sm m-0 mt-2 text-charcoal">
            {hasSavings
              ? t("insight.timeSaved.detail", { baseline: baselineWpm })
              : t("insight.timeSaved.empty")}
          </p>
        </div>
        <span
          className="inline-flex size-10 shrink-0 items-center justify-center rounded-lg border border-hairline-strong bg-surface-elevated text-accent-green"
          aria-hidden
        >
          <Clock size={18} strokeWidth={1.75} />
        </span>
      </div>
      <p className="text-caption relative m-0 mt-4 text-ash">
        {t("insight.timeSaved.method")}
      </p>
    </div>
  );
}
