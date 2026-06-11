import { glowSurfaceClasses } from "../layout/glowSurface";

interface InsightMetricCardProps {
  label: string;
  value: string;
  detail?: string;
  glow?: "green" | "blue" | "red" | "orange";
}

export function InsightMetricCard({
  label,
  value,
  detail,
  glow = "blue",
}: InsightMetricCardProps) {
  return (
    <div
      className={[
        glowSurfaceClasses(glow),
        "rounded-lg border border-hairline-strong bg-surface-card px-4 py-3 sm:px-5 sm:py-4",
      ].join(" ")}
    >
      <p className="text-caption relative m-0 text-charcoal">{label}</p>
      <p className="text-heading-sm relative m-0 mt-1 text-ink sm:text-heading-md">
        {value}
      </p>
      {detail && (
        <p className="text-body-sm relative mt-2 text-ash">{detail}</p>
      )}
    </div>
  );
}
