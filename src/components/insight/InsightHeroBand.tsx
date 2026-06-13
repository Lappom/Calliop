import type { LatencyMetricsPayload } from "../../hooks/usePipelineState";
import { useUiLocale } from "../../i18n/useUiLocale";
import { glowSurfaceClasses } from "../layout/glowSurface";
import { AnimatedMetricValue } from "./AnimatedMetricValue";
import { formatLatencyDetail } from "./insightUtils";

interface InsightHeroBandProps {
  wordsToday: number;
  dictationsToday: number;
  activeLatency: LatencyMetricsPayload | null;
  learnedCount: number;
}

export function InsightHeroBand({
  wordsToday,
  dictationsToday,
  activeLatency,
  learnedCount,
}: InsightHeroBandProps) {
  const { t, formatNumber } = useUiLocale();
  const hasLatency = activeLatency !== null;

  return (
    <div
      className={[
        glowSurfaceClasses("blue"),
        "insight-metric-hover rounded-lg border border-hairline-strong bg-surface-card p-6 sm:p-8",
      ].join(" ")}
    >
      <div className="relative flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
        <div className="min-w-0">
          <p className="text-caption m-0 text-charcoal">
            {t("insight.metrics.wordsToday.label")}
          </p>
          <p className="text-display-serif m-0 mt-2 text-4xl text-ink sm:text-5xl">
            <AnimatedMetricValue
              value={wordsToday}
              format={(n) => formatNumber(Math.round(n))}
              className="font-[family-name:var(--font-mono)] tabular-nums"
            />
          </p>
          <p className="text-body-sm m-0 mt-2 text-charcoal">
            {wordsToday > 0
              ? t("insight.metrics.wordsToday.dictations", {
                  count: dictationsToday,
                })
              : t("insight.metrics.wordsToday.empty")}
          </p>
        </div>

        <div className="flex flex-wrap gap-3 lg:justify-end">
          <HeroPill
            label={t("insight.metrics.lastLatency.label")}
            value={
              hasLatency ? `${activeLatency.totalMs} ms` : t("common.emDash")
            }
            detail={
              hasLatency
                ? formatLatencyDetail(activeLatency, t)
                : t("insight.metrics.lastLatency.empty")
            }
          />
          <HeroPill
            label={t("insight.metrics.learned.label")}
            value={String(learnedCount)}
            detail={
              learnedCount > 0
                ? t("insight.metrics.learned.detail", { count: learnedCount })
                : t("insight.metrics.learned.empty")
            }
            numericValue={learnedCount}
          />
        </div>
      </div>
    </div>
  );
}

function HeroPill({
  label,
  value,
  detail,
  numericValue,
}: {
  label: string;
  value: string;
  detail: string;
  numericValue?: number;
}) {
  return (
    <div className="min-w-[140px] flex-1 rounded-lg border border-hairline bg-surface-elevated px-4 py-3 sm:min-w-[180px] sm:flex-none">
      <p className="text-caption m-0 text-charcoal">{label}</p>
      <p className="text-heading-sm m-0 mt-1 font-[family-name:var(--font-mono)] tabular-nums text-ink">
        {numericValue != null ? (
          <AnimatedMetricValue value={numericValue} />
        ) : (
          value
        )}
      </p>
      <p className="text-caption m-0 mt-1.5 text-ash">{detail}</p>
    </div>
  );
}
