import type { LatencyMetricsPayload } from "../../hooks/usePipelineState";
import { useDictionary } from "../../hooks/useDictionary";
import { Card } from "../ui/Card";
import { SectionGlow } from "../layout/SectionGlow";

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

export function InsightView({ latencyMetrics }: InsightViewProps) {
  const { words, loaded } = useDictionary();
  const learnedCount = words.filter((w) => w.source === "learned").length;
  const hasLatency = latencyMetrics !== null;
  const hasAnyData = hasLatency || learnedCount > 0;

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">Insight</h1>
        <p className="text-body-sm text-charcoal">
          Statistiques d&apos;usage et performances de votre dictée locale.
        </p>
      </header>

      <SectionGlow glow="blue">
        <div className="grid gap-4 sm:grid-cols-3">
          <MetricCard
            label="Latence dernière dictée"
            value={
              hasLatency
                ? `${latencyMetrics.totalMs} ms`
                : "—"
            }
            detail={
              hasLatency
                ? `STT ${latencyMetrics.sttMs} ms · injection ${latencyMetrics.injectMs} ms${
                    latencyMetrics.llmMs > 0
                      ? ` · LLM ${latencyMetrics.llmMs} ms`
                      : ""
                  }`
                : "Effectuez une dictée pour mesurer la latence."
            }
          />
          <MetricCard
            label="Mots dictés aujourd'hui"
            value="0"
            detail="Historique SQLite — Phase 3f."
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

        {!hasAnyData && (
          <p className="text-body-sm mt-6 text-charcoal">
            Les statistiques apparaîtront après vos premières dictées.
          </p>
        )}
      </SectionGlow>
    </div>
  );
}
