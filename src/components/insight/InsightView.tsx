import { useMemo } from "react";
import { useUiLocale } from "../../i18n/useUiLocale";
import type { LatencyMetricsPayload } from "../../hooks/usePipelineState";
import { useInsights } from "../../hooks/useInsights";
import { useRefreshSpin } from "../../hooks/useRefreshSpin";
import { SnippetListToolbarButton } from "../snippets/SnippetListToolbarButton";
import { RefreshIcon } from "../ui/RefreshIcon";
import { glowSurfaceClasses } from "../layout/glowSurface";
import { ActivityChart } from "./charts/ActivityChart";
import { AppUsageDonut } from "./charts/AppUsageDonut";
import { LatencyChart } from "./charts/LatencyChart";
import { WpmGauge } from "./charts/WpmGauge";
import { InsightChartPanel } from "./InsightChartPanel";
import { InsightEmptyState } from "./InsightEmptyState";
import { InsightMetricCard } from "./InsightMetricCard";
import { InsightWeekSummary } from "./InsightWeekSummary";
import {
  computeWeekSummary,
  formatAudioDuration,
  formatLatencyDetail,
  hasInsightData,
  resolveActiveLatency,
} from "./insightUtils";

interface InsightViewProps {
  latencyMetrics: LatencyMetricsPayload | null;
}

export function InsightView({ latencyMetrics }: InsightViewProps) {
  const { t, formatNumber } = useUiLocale();
  const { insights, loaded, errorMessage, reload } = useInsights();
  const { spinning: refreshSpinning, runRefresh } = useRefreshSpin();

  const activeLatency = resolveActiveLatency(
    latencyMetrics,
    insights?.lastLatency,
  );
  const weekSummary = useMemo(
    () => computeWeekSummary(insights?.dailyActivity ?? []),
    [insights?.dailyActivity],
  );

  const hasLatency = activeLatency !== null;
  const wordsToday = insights?.wordsToday ?? 0;
  const dictationsToday = insights?.dictationsToday ?? 0;
  const totalWords = insights?.totalWords ?? 0;
  const totalDictations = insights?.totalDictations ?? 0;
  const learnedCount = insights?.learnedCorrections ?? 0;
  const averageWpm = insights?.averageWpm ?? 0;
  const wpmPercent = insights?.wpmVsTypingPercent ?? 0;
  const averageLatencyMs = insights?.averageLatencyMs ?? 0;
  const totalAudioMinutes = insights?.totalAudioMinutes ?? 0;
  const appUsage = insights?.appUsage ?? [];
  const dailyActivity = insights?.dailyActivity ?? [];
  const recentLatency = insights?.recentLatency ?? [];
  const hasActivityData = dailyActivity.some((day) => day.wordCount > 0);
  const showEmptyState = loaded && !hasInsightData(insights);

  return (
    <div className="flex flex-col gap-8">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-heading-md mb-2 text-ink">{t("insight.title")}</h1>
          <p className="text-body-sm text-charcoal">{t("insight.subtitle")}</p>
        </div>
        {loaded && (
          <SnippetListToolbarButton
            label={t("insight.refresh")}
            disabled={refreshSpinning}
            onClick={() => {
              void runRefresh(() => reload());
            }}
          >
            <RefreshIcon spinning={refreshSpinning} />
          </SnippetListToolbarButton>
        )}
      </header>

      {errorMessage && (
        <p className="text-body-sm text-accent-red">{errorMessage}</p>
      )}

      {!loaded && (
        <p className="text-body-sm text-charcoal">{t("common.loading")}</p>
      )}

      {showEmptyState && <InsightEmptyState />}

      {loaded && hasInsightData(insights) && (
        <>
          <section className="flex flex-col gap-3">
            <p className="text-caption m-0 text-charcoal">
              {t("insight.sections.today")}
            </p>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              <InsightMetricCard
                label={t("insight.metrics.lastLatency.label")}
                value={
                  hasLatency
                    ? `${activeLatency.totalMs} ms`
                    : t("common.emDash")
                }
                detail={
                  hasLatency
                    ? formatLatencyDetail(activeLatency, t)
                    : t("insight.metrics.lastLatency.empty")
                }
                glow="blue"
              />
              <InsightMetricCard
                label={t("insight.metrics.wordsToday.label")}
                value={String(wordsToday)}
                detail={
                  wordsToday > 0
                    ? t("insight.metrics.wordsToday.dictations", {
                        count: dictationsToday,
                      })
                    : t("insight.metrics.wordsToday.empty")
                }
                glow="green"
              />
              <InsightMetricCard
                label={t("insight.metrics.learned.label")}
                value={String(learnedCount)}
                detail={
                  learnedCount > 0
                    ? t("insight.metrics.learned.detail", {
                        count: learnedCount,
                      })
                    : t("insight.metrics.learned.empty")
                }
                glow="orange"
              />
            </div>
          </section>

          <section className="flex flex-col gap-3">
            <p className="text-caption m-0 text-charcoal">
              {t("insight.sections.global")}
            </p>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
              <InsightMetricCard
                label={t("insight.metrics.totalWords.label")}
                value={formatNumber(totalWords)}
                glow="blue"
              />
              <InsightMetricCard
                label={t("insight.metrics.totalDictations.label")}
                value={String(totalDictations)}
                glow="green"
              />
              <InsightMetricCard
                label={t("insight.metrics.avgLatency.label")}
                value={
                  averageLatencyMs > 0
                    ? `${averageLatencyMs} ms`
                    : t("common.emDash")
                }
                glow="orange"
              />
              <InsightMetricCard
                label={t("insight.metrics.speechTime.label")}
                value={formatAudioDuration(totalAudioMinutes, t)}
                detail={t("insight.metrics.speechTime.detail")}
                glow="blue"
              />
            </div>
          </section>

          {hasActivityData && <InsightWeekSummary summary={weekSummary} />}

          <div className="grid gap-4 lg:grid-cols-2 lg:items-stretch">
            <InsightChartPanel
              title={t("insight.charts.activity.title")}
              description={t("insight.charts.activity.description")}
              empty={!hasActivityData}
              emptyMessage={t("insight.charts.activity.empty")}
              glow="blue"
            >
              <ActivityChart data={dailyActivity} />
            </InsightChartPanel>

            <InsightChartPanel
              title={t("insight.charts.latency.title")}
              description={t("insight.charts.latency.description")}
              empty={recentLatency.length === 0}
              emptyMessage={t("insight.charts.latency.empty")}
              glow="orange"
            >
              <LatencyChart data={recentLatency} />
            </InsightChartPanel>
          </div>

          <div className="grid gap-4 lg:grid-cols-[1fr_260px] lg:items-stretch">
            <InsightChartPanel
              title={t("insight.charts.appUsage.title")}
              description={t("insight.charts.appUsage.description")}
              empty={appUsage.length === 0}
              emptyMessage={t("insight.charts.appUsage.empty")}
              glow="green"
              className="min-h-[280px] sm:min-h-[320px]"
            >
              <AppUsageDonut data={appUsage} />
            </InsightChartPanel>

            <div
              className={[
                glowSurfaceClasses("orange"),
                "flex h-full min-h-[280px] flex-col justify-between rounded-lg border border-hairline-strong bg-surface-card p-4 sm:min-h-[320px] sm:p-6",
              ].join(" ")}
            >
              <div className="relative">
                <h2 className="text-heading-sm m-0 text-ink">
                  {t("insight.wpm.title")}
                </h2>
                <p className="text-body-sm mt-2 text-charcoal">
                  {t("insight.wpm.description")}
                </p>
              </div>
              <div className="relative flex flex-1 flex-col items-center justify-center overflow-visible py-4">
                <WpmGauge percent={wpmPercent} averageWpm={averageWpm} />
              </div>
              <p className="text-body-sm relative m-0 text-center text-charcoal">
                {averageWpm > 0
                  ? t("insight.wpm.average", { wpm: Math.round(averageWpm) })
                  : t("insight.wpm.empty")}
              </p>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
