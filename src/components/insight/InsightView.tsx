import { RefreshCw } from "lucide-react";
import { useMemo } from "react";
import type { LatencyMetricsPayload } from "../../hooks/usePipelineState";
import { useInsights } from "../../hooks/useInsights";
import { SnippetListToolbarButton } from "../snippets/SnippetListToolbarButton";
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
  const { insights, loaded, errorMessage, reload } = useInsights();

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
          <h1 className="text-heading-md mb-2 text-ink">Statistiques</h1>
          <p className="text-body-sm text-charcoal">
            Activité, performances du pipeline et répartition par application —
            calculées localement depuis votre historique.
          </p>
        </div>
        {loaded && (
          <SnippetListToolbarButton
            label="Actualiser les statistiques"
            onClick={() => {
              void reload();
            }}
          >
            <RefreshCw size={16} strokeWidth={1.75} />
          </SnippetListToolbarButton>
        )}
      </header>

      {errorMessage && (
        <p className="text-body-sm text-accent-red">{errorMessage}</p>
      )}

      {!loaded && (
        <p className="text-body-sm text-charcoal">Chargement…</p>
      )}

      {showEmptyState && <InsightEmptyState />}

      {loaded && hasInsightData(insights) && (
        <>
          <section className="flex flex-col gap-3">
            <p className="text-caption m-0 text-charcoal">Aujourd&apos;hui</p>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              <InsightMetricCard
                label="Latence dernière dictée"
                value={hasLatency ? `${activeLatency.totalMs} ms` : "—"}
                detail={
                  hasLatency
                    ? formatLatencyDetail(activeLatency)
                    : "Effectuez une dictée pour mesurer la latence."
                }
                glow="blue"
              />
              <InsightMetricCard
                label="Mots dictés"
                value={String(wordsToday)}
                detail={
                  wordsToday > 0
                    ? `${dictationsToday} dictée${dictationsToday > 1 ? "s" : ""} depuis minuit.`
                    : "Comptés depuis minuit (heure locale)."
                }
                glow="green"
              />
              <InsightMetricCard
                label="Corrections apprises"
                value={String(learnedCount)}
                detail={
                  learnedCount > 0
                    ? `${learnedCount} mot${learnedCount > 1 ? "s" : ""} ajouté${learnedCount > 1 ? "s" : ""} au dictionnaire.`
                    : "Les corrections enrichissent le dictionnaire local."
                }
                glow="orange"
              />
            </div>
          </section>

          <section className="flex flex-col gap-3">
            <p className="text-caption m-0 text-charcoal">Global</p>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
              <InsightMetricCard
                label="Mots transcrits"
                value={totalWords.toLocaleString("fr-FR")}
                glow="blue"
              />
              <InsightMetricCard
                label="Dictées enregistrées"
                value={String(totalDictations)}
                glow="green"
              />
              <InsightMetricCard
                label="Latence moyenne"
                value={averageLatencyMs > 0 ? `${averageLatencyMs} ms` : "—"}
                glow="orange"
              />
              <InsightMetricCard
                label="Temps de parole"
                value={formatAudioDuration(totalAudioMinutes)}
                detail="Durée audio cumulée de vos dictées."
                glow="blue"
              />
            </div>
          </section>

          {hasActivityData && <InsightWeekSummary summary={weekSummary} />}

          <div className="grid gap-4 lg:grid-cols-2 lg:items-stretch">
            <InsightChartPanel
              title="Activité — 7 jours"
              description="Mots dictés par jour. Le nombre de dictées apparaît au-dessus de chaque barre."
              empty={!hasActivityData}
              emptyMessage="Les barres apparaîtront après vos premières dictées."
              glow="blue"
            >
              <ActivityChart data={dailyActivity} />
            </InsightChartPanel>

            <InsightChartPanel
              title="Latence — dernières dictées"
              description="Décomposition STT, LLM et injection (empilé)."
              empty={recentLatency.length === 0}
              emptyMessage="Aucune mesure de latence enregistrée."
              glow="orange"
            >
              <LatencyChart data={recentLatency} />
            </InsightChartPanel>
          </div>

          <div className="grid gap-4 lg:grid-cols-[1fr_260px] lg:items-stretch">
            <InsightChartPanel
              title="Utilisation par application"
              description="Répartition des mots dictés selon la fenêtre active."
              empty={appUsage.length === 0}
              emptyMessage="Aucune donnée de contexte pour le moment."
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
                <h2 className="text-heading-sm m-0 text-ink">Vitesse vs frappe</h2>
                <p className="text-body-sm mt-2 text-charcoal">
                  Comparaison à 40 mots/min (frappe clavier moyenne).
                </p>
              </div>
              <div className="relative flex flex-1 flex-col items-center justify-center overflow-visible py-4">
                <WpmGauge percent={wpmPercent} averageWpm={averageWpm} />
              </div>
              <p className="text-body-sm relative m-0 text-center text-charcoal">
                {averageWpm > 0
                  ? `${Math.round(averageWpm)} mots/min en moyenne`
                  : "Vitesse calculée après vos premières dictées."}
              </p>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
