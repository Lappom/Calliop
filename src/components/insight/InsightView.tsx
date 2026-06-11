import type { ReactNode } from "react";
import type { LatencyMetricsPayload } from "../../hooks/usePipelineState";
import { useInsights } from "../../hooks/useInsights";
import { Card } from "../ui/Card";
import { SectionGlow } from "../layout/SectionGlow";
import { ActivityChart } from "./charts/ActivityChart";
import { AppUsageDonut } from "./charts/AppUsageDonut";
import { LatencyChart } from "./charts/LatencyChart";
import { WpmGauge } from "./charts/WpmGauge";

interface InsightViewProps {
  latencyMetrics: LatencyMetricsPayload | null;
}

function MetricCard({
  label,
  value,
  detail,
}: {
  label: string;
  value: string;
  detail?: string;
}) {
  return (
    <Card variant="bordered" className="p-5">
      <p className="text-caption mb-2 text-charcoal">{label}</p>
      <p className="text-heading-md m-0 text-ink">{value}</p>
      {detail && (
        <p className="text-body-sm mt-2 text-ash">{detail}</p>
      )}
    </Card>
  );
}

function formatLatencyDetail(
  latency: {
    sttMs: number;
    sttWaitMs?: number;
    llmMs: number;
    llmBlockedMs?: number;
    injectMs: number;
  },
): string {
  const sttLabel =
    latency.sttWaitMs != null
      ? `STT attente ${latency.sttWaitMs} ms (inférence ${latency.sttMs} ms)`
      : `STT ${latency.sttMs} ms`;
  const parts = [sttLabel, `injection ${latency.injectMs} ms`];
  if (latency.llmBlockedMs != null && latency.llmBlockedMs > 0) {
    parts.push(`LLM bloqué ${latency.llmBlockedMs} ms`);
  } else if (latency.llmMs > 0) {
    parts.push(`LLM ${latency.llmMs} ms`);
  }
  return parts.join(" · ");
}

function ChartPanel({
  title,
  description,
  children,
  empty,
  emptyMessage,
  className = "",
}: {
  title: string;
  description: string;
  children: ReactNode;
  empty: boolean;
  emptyMessage: string;
  className?: string;
}) {
  return (
    <Card
      variant="bordered"
      className={[
        "flex h-full min-h-[360px] flex-col gap-4 p-6",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className="shrink-0">
        <h2 className="text-heading-sm m-0 text-ink">{title}</h2>
        <p className="text-body-sm mt-2 text-charcoal">{description}</p>
      </div>
      {empty ? (
        <p className="text-body-sm flex-1 text-charcoal">{emptyMessage}</p>
      ) : (
        <div className="flex flex-1 flex-col justify-end">{children}</div>
      )}
    </Card>
  );
}

export function InsightView({ latencyMetrics }: InsightViewProps) {
  const { insights, loaded, errorMessage } = useInsights();

  const sessionLatency = latencyMetrics;
  const storedLatency = insights?.lastLatency ?? null;
  const activeLatency =
    sessionLatency ??
    (storedLatency
      ? {
          sttMs: storedLatency.sttMs,
          llmMs: storedLatency.llmMs,
          injectMs: storedLatency.injectMs,
          totalMs: storedLatency.totalMs,
        }
      : null);

  const hasLatency = activeLatency !== null;
  const wordsToday = insights?.wordsToday ?? 0;
  const totalWords = insights?.totalWords ?? 0;
  const learnedCount = insights?.learnedCorrections ?? 0;
  const averageWpm = insights?.averageWpm ?? 0;
  const wpmPercent = insights?.wpmVsTypingPercent ?? 0;
  const appUsage = insights?.appUsage ?? [];
  const dailyActivity = insights?.dailyActivity ?? [];
  const recentLatency = insights?.recentLatency ?? [];
  const hasActivityData = dailyActivity.some((d) => d.wordCount > 0);
  const hasAnyData =
    hasLatency || wordsToday > 0 || totalWords > 0 || learnedCount > 0;

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">Insight</h1>
        <p className="text-body-sm text-charcoal">
          Statistiques d&apos;usage et performances de votre dictée locale.
        </p>
      </header>

      {errorMessage && (
        <p className="text-body-sm text-accent-red">{errorMessage}</p>
      )}

      <SectionGlow glow="blue">
        <div className="grid gap-4 sm:grid-cols-3">
          <MetricCard
            label="Latence dernière dictée"
            value={hasLatency ? `${activeLatency.totalMs} ms` : "—"}
            detail={
              hasLatency
                ? formatLatencyDetail(activeLatency)
                : "Effectuez une dictée pour mesurer la latence."
            }
          />
          <MetricCard
            label="Mots dictés aujourd'hui"
            value={loaded ? String(wordsToday) : "—"}
            detail={
              wordsToday > 0
                ? "Comptés depuis minuit (heure locale)."
                : "Vos dictées du jour apparaîtront ici."
            }
          />
          <MetricCard
            label="Corrections apprises"
            value={loaded ? String(learnedCount) : "—"}
            detail={
              learnedCount > 0
                ? `${learnedCount} mot${learnedCount > 1 ? "s" : ""} ajouté${learnedCount > 1 ? "s" : ""} automatiquement au dictionnaire.`
                : "Les corrections manuelles enrichissent le dictionnaire."
            }
          />
        </div>
      </SectionGlow>

      <div className="grid gap-4 lg:grid-cols-2 lg:items-stretch">
        <SectionGlow glow="blue" className="h-full">
          <ChartPanel
            title="Activité — 7 jours"
            description="Nombre de mots dictés par jour."
            empty={!loaded || !hasActivityData}
            emptyMessage="Les barres apparaîtront après vos premières dictées."
          >
            <ActivityChart data={dailyActivity} />
          </ChartPanel>
        </SectionGlow>

        <SectionGlow glow="orange" className="h-full">
          <ChartPanel
            title="Latence — dernières dictées"
            description="Décomposition STT, LLM et injection (empilé)."
            empty={!loaded || recentLatency.length === 0}
            emptyMessage="Aucune mesure de latence enregistrée."
          >
            <LatencyChart data={recentLatency} />
          </ChartPanel>
        </SectionGlow>
      </div>

      <div className="grid gap-4 lg:grid-cols-[1fr_240px] lg:items-stretch">
        <SectionGlow glow="green" className="h-full">
          <ChartPanel
            title="Utilisation par application"
            description="Répartition des mots dictés selon la fenêtre active."
            empty={!loaded || appUsage.length === 0}
            emptyMessage="Aucune donnée de contexte pour le moment."
            className="min-h-[320px]"
          >
            <AppUsageDonut data={appUsage} />
          </ChartPanel>
        </SectionGlow>

        <Card
          variant="bordered"
          className="flex h-full min-h-[320px] flex-col justify-between p-6"
        >
          <h2 className="text-heading-sm m-0 text-ink">Vitesse vs frappe</h2>
          <div className="flex flex-1 flex-col items-center justify-center overflow-visible py-4">
            <WpmGauge percent={wpmPercent} averageWpm={averageWpm} />
          </div>
          <p className="text-body-sm m-0 text-center text-charcoal">
            Total :{" "}
            <span className="text-ink">
              {loaded ? totalWords.toLocaleString("fr-FR") : "—"}
            </span>{" "}
            mots dictés
          </p>
        </Card>
      </div>

      {!hasAnyData && loaded && (
        <p className="text-body-sm text-charcoal">
          Les statistiques apparaîtront après vos premières dictées.
        </p>
      )}
    </div>
  );
}
