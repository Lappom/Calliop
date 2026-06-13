import { useEffect, useMemo, useState } from "react";
import { MOTION_STAGGER } from "../../lib/motion/presets";
import { useUiLocale } from "../../i18n/useUiLocale";
import type { LatencyMetricsPayload } from "../../hooks/usePipelineState";
import { useInsights } from "../../hooks/useInsights";
import { useRefreshSpin } from "../../hooks/useRefreshSpin";
import { SectionGlow } from "../layout/SectionGlow";
import { Stagger } from "../motion/Stagger";
import { SnippetListToolbarButton } from "../snippets/SnippetListToolbarButton";
import { RefreshIcon } from "../ui/RefreshIcon";
import { ActivityChart } from "./charts/ActivityChart";
import { AppUsageDonut } from "./charts/AppUsageDonut";
import { HourAppHeatmapChart } from "./charts/HourAppHeatmapChart";
import { LatencyChart } from "./charts/LatencyChart";
import { WpmGauge } from "./charts/WpmGauge";
import { InsightChartPanel } from "./InsightChartPanel";
import { InsightEmptyState } from "./InsightEmptyState";
import { InsightHeroBand } from "./InsightHeroBand";
import { InsightLoadingSkeleton } from "./InsightLoadingSkeleton";
import { InsightMetricCard } from "./InsightMetricCard";
import { InsightMetricStrip } from "./InsightMetricStrip";
import { InsightStreakCard } from "./InsightStreakCard";
import {
  InsightTabNav,
} from "./InsightTabNav";
import { InsightTabPanel } from "./InsightTabPanel";
import { readInsightTabFromHash } from "./insightTabs";
import { InsightTimeSavedCard } from "./InsightTimeSavedCard";
import {
  computeWeekSummary,
  formatAudioDuration,
  hasHeatmapData,
  hasInsightData,
  resolveActiveLatency,
} from "./insightUtils";
import type { InsightTabId } from "./insightTabs";

interface InsightViewProps {
  latencyMetrics: LatencyMetricsPayload | null;
}

export function InsightView({ latencyMetrics }: InsightViewProps) {
  const { t, formatNumber } = useUiLocale();
  const { insights, loaded, errorMessage, reload } = useInsights();
  const { spinning: refreshSpinning, runRefresh } = useRefreshSpin();
  const [animateKey, setAnimateKey] = useState(0);
  const [activeTab, setActiveTab] = useState<InsightTabId>(
    () => readInsightTabFromHash() ?? "overview",
  );

  useEffect(() => {
    const hash = `#${activeTab}`;
    if (window.location.hash !== hash) {
      window.history.replaceState(null, "", hash);
    }
  }, [activeTab]);

  const activeLatency = resolveActiveLatency(
    latencyMetrics,
    insights?.lastLatency,
  );
  const weekSummary = useMemo(
    () => computeWeekSummary(insights?.dailyActivity ?? []),
    [insights?.dailyActivity],
  );

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
  const hourAppHeatmap = insights?.hourAppHeatmap ?? [];
  const streak = insights?.streak ?? {
    currentStreak: 0,
    bestStreak: 0,
    activeToday: false,
  };
  const timeSaved = insights?.timeSaved ?? {
    minutesSaved: 0,
    baselineWpm: 40,
  };
  const hasActivityData = dailyActivity.some((day) => day.wordCount > 0);
  const hasHeatmap = hasHeatmapData(hourAppHeatmap);
  const showEmptyState = loaded && !hasInsightData(insights);

  const handleRefresh = () => {
    void runRefresh(async () => {
      setAnimateKey((key) => key + 1);
      await reload();
    });
  };

  return (
    <Stagger
      className="flex flex-col gap-8"
      itemMotion="fadeUp"
      staggerDelay={MOTION_STAGGER.editorial}
    >
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-display-serif mb-2 text-4xl text-ink sm:text-5xl">
            {t("insight.title")}
          </h1>
          <p className="text-body-sm text-charcoal">{t("insight.subtitle")}</p>
        </div>
        {loaded && (
          <SnippetListToolbarButton
            label={t("insight.refresh")}
            disabled={refreshSpinning}
            onClick={handleRefresh}
          >
            <RefreshIcon spinning={refreshSpinning} />
          </SnippetListToolbarButton>
        )}
      </header>

      {errorMessage && (
        <p className="text-body-sm text-accent-red">{errorMessage}</p>
      )}

      {!loaded && <InsightLoadingSkeleton />}

      {showEmptyState && <InsightEmptyState />}

      {loaded && hasInsightData(insights) && (
        <>
          <InsightTabNav activeTab={activeTab} onChange={setActiveTab} />

          <InsightTabPanel activeTab={activeTab}>
            {activeTab === "overview" && (
              <>
                <InsightHeroBand
                  wordsToday={wordsToday}
                  dictationsToday={dictationsToday}
                  activeLatency={activeLatency}
                  learnedCount={learnedCount}
                />
                <Stagger className="grid gap-4 lg:grid-cols-2" itemMotion="fade">
                  <InsightStreakCard streak={streak} />
                  <InsightTimeSavedCard timeSaved={timeSaved} />
                </Stagger>
              </>
            )}

            {activeTab === "activity" && (
              <SectionGlow glow="green" className="flex flex-col gap-4">
                <InsightChartPanel
                  title={t("insight.charts.activity.title")}
                  description={t("insight.charts.activity.description")}
                  empty={!hasActivityData}
                  emptyMessage={t("insight.charts.activity.empty")}
                  glow="green"
                  animateKey={animateKey}
                >
                  <ActivityChart data={dailyActivity} />
                </InsightChartPanel>
                {hasActivityData && (
                  <Stagger className="flex flex-col gap-4" itemMotion="fade">
                    <InsightMetricStrip summary={weekSummary} />
                    <InsightChartPanel
                      title={t("insight.charts.heatmap.title")}
                      description={t("insight.charts.heatmap.description")}
                      empty={!hasHeatmap}
                      emptyMessage={t("insight.charts.heatmap.empty")}
                      glow="green"
                      animateKey={animateKey}
                      className="min-h-0 sm:min-h-0"
                    >
                      <HourAppHeatmapChart data={hourAppHeatmap} />
                    </InsightChartPanel>
                  </Stagger>
                )}
              </SectionGlow>
            )}

            {activeTab === "performance" && (
              <SectionGlow glow="orange" className="flex flex-col gap-4">
                <div className="grid gap-4 lg:grid-cols-2 lg:items-stretch">
                  <InsightChartPanel
                    title={t("insight.charts.latency.title")}
                    description={t("insight.charts.latency.description")}
                    empty={recentLatency.length === 0}
                    emptyMessage={t("insight.charts.latency.empty")}
                    glow="orange"
                    animateKey={animateKey}
                    footer={
                      averageLatencyMs > 0 ? (
                        <p className="text-caption m-0 text-charcoal">
                          {t("insight.metrics.avgLatency.label")}:{" "}
                          <span className="font-[family-name:var(--font-mono)] tabular-nums text-ink">
                            {averageLatencyMs} ms
                          </span>
                        </p>
                      ) : undefined
                    }
                  >
                    <LatencyChart data={recentLatency} />
                  </InsightChartPanel>

                  <InsightChartPanel
                    title={t("insight.wpm.title")}
                    description={t("insight.wpm.description")}
                    empty={averageWpm <= 0 && wpmPercent <= 0}
                    emptyMessage={t("insight.wpm.empty")}
                    glow="orange"
                    animateKey={animateKey}
                    className="min-h-[280px] sm:min-h-[320px]"
                    footer={
                      averageWpm > 0 ? (
                        <p className="text-body-sm m-0 text-center text-charcoal">
                          {t("insight.wpm.average", {
                            wpm: Math.round(averageWpm),
                          })}
                        </p>
                      ) : undefined
                    }
                  >
                    <div className="flex flex-1 flex-col items-center justify-center overflow-visible py-4">
                      <WpmGauge percent={wpmPercent} averageWpm={averageWpm} />
                    </div>
                  </InsightChartPanel>
                </div>
              </SectionGlow>
            )}

            {activeTab === "global" && (
              <section className="flex flex-col gap-4">
                <Stagger
                  className="grid items-stretch gap-3 sm:grid-cols-2 lg:grid-cols-4"
                  itemMotion="fade"
                  itemClassName="h-full"
                >
                  <InsightMetricCard
                    label={t("insight.metrics.totalWords.label")}
                    value={formatNumber(totalWords)}
                    numericValue={totalWords}
                    formatValue={(n) => formatNumber(Math.round(n))}
                    variant="compact"
                    className="h-full"
                  />
                  <InsightMetricCard
                    label={t("insight.metrics.totalDictations.label")}
                    value={String(totalDictations)}
                    numericValue={totalDictations}
                    variant="compact"
                    className="h-full"
                  />
                  <InsightMetricCard
                    label={t("insight.metrics.speechTime.label")}
                    value={formatAudioDuration(totalAudioMinutes, t)}
                    detail={t("insight.metrics.speechTime.detail")}
                    variant="compact"
                    className="h-full"
                  />
                  <InsightMetricCard
                    label={t("insight.metrics.avgLatency.label")}
                    value={
                      averageLatencyMs > 0
                        ? `${averageLatencyMs} ms`
                        : t("common.emDash")
                    }
                    numericValue={
                      averageLatencyMs > 0 ? averageLatencyMs : undefined
                    }
                    formatValue={(n) => `${Math.round(n)} ms`}
                    variant="compact"
                    className="h-full"
                  />
                </Stagger>

                <InsightChartPanel
                  title={t("insight.charts.appUsage.title")}
                  description={t("insight.charts.appUsage.description")}
                  empty={appUsage.length === 0}
                  emptyMessage={t("insight.charts.appUsage.empty")}
                  glow="green"
                  animateKey={animateKey}
                  className="min-h-[280px]"
                >
                  <AppUsageDonut data={appUsage} />
                </InsightChartPanel>
              </section>
            )}
          </InsightTabPanel>
        </>
      )}
    </Stagger>
  );
}
