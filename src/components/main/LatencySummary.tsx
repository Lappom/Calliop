import { useTranslation } from "react-i18next";
import type { LatencyMetricsPayload } from "../../hooks/usePipelineState";
import { glowSurfaceClasses } from "../layout/glowSurface";
import { formatLatencyBreakdown } from "./mainUtils";

interface LatencySummaryProps {
  metrics: LatencyMetricsPayload;
}

export function LatencySummary({ metrics }: LatencySummaryProps) {
  const { t } = useTranslation();
  const breakdown = formatLatencyBreakdown(metrics);

  return (
    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      <MetricPill label={t("main.latency.stt")} value={breakdown.stt} glow="blue" />
      {breakdown.llm && (
        <MetricPill
          label={t("main.latency.llm")}
          value={breakdown.llm}
          glow="orange"
        />
      )}
      <MetricPill
        label={t("main.latency.inject")}
        value={breakdown.inject}
        glow="green"
      />
      <MetricPill label={t("main.latency.total")} value={breakdown.total} glow="blue" />
    </div>
  );
}

function MetricPill({
  label,
  value,
  glow,
}: {
  label: string;
  value: string;
  glow: "green" | "blue" | "orange";
}) {
  return (
    <div
      className={[
        glowSurfaceClasses(glow),
        "rounded-lg border border-hairline-strong bg-surface-card px-4 py-3",
      ].join(" ")}
    >
      <p className="text-caption relative m-0 text-charcoal">{label}</p>
      <p className="text-heading-sm relative m-0 mt-1 font-[family-name:var(--font-ui)] text-ink">
        {value}
      </p>
    </div>
  );
}
