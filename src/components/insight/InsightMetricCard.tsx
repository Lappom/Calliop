import type { ReactNode } from "react";
import { glowSurfaceClasses, type GlowColor } from "../layout/glowSurface";
import { AnimatedMetricValue } from "./AnimatedMetricValue";

type InsightMetricVariant = "default" | "compact";

interface InsightMetricCardProps {
  label: string;
  value: string;
  detail?: string;
  glow?: GlowColor;
  variant?: InsightMetricVariant;
  /** When set, value animates with count-up. */
  numericValue?: number;
  formatValue?: (value: number) => string;
  className?: string;
}

const variantClasses: Record<InsightMetricVariant, string> = {
  default: "px-4 py-3 sm:px-5 sm:py-4",
  compact: "px-3 py-2.5 sm:px-4 sm:py-3",
};

const valueClasses: Record<InsightMetricVariant, string> = {
  default: "text-heading-sm sm:text-heading-md",
  compact: "text-body-md",
};

export function InsightMetricCard({
  label,
  value,
  detail,
  glow = "blue",
  variant = "default",
  numericValue,
  formatValue,
  className = "",
}: InsightMetricCardProps) {
  const useGlow = variant === "default";

  return (
    <div
      className={[
        useGlow ? glowSurfaceClasses(glow) : "",
        "insight-metric-hover rounded-lg border border-hairline-strong bg-surface-card",
        variantClasses[variant],
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <p className="text-caption relative m-0 text-charcoal">{label}</p>
      <p
        className={[
          "relative m-0 mt-1 font-[family-name:var(--font-mono)] tabular-nums text-ink",
          valueClasses[variant],
        ].join(" ")}
      >
        {numericValue != null ? (
          <AnimatedMetricValue
            value={numericValue}
            format={formatValue}
          />
        ) : (
          value
        )}
      </p>
      {(detail || variant === "compact") && (
        <p
          className={[
            "text-body-sm relative mt-2 min-h-[1.25rem] text-ash",
            variant === "compact" && !detail ? "invisible" : "",
          ].join(" ")}
          aria-hidden={variant === "compact" && !detail ? true : undefined}
        >
          {detail ?? "\u00a0"}
        </p>
      )}
    </div>
  );
}

/** Section heading for insight bands. */
export function InsightSectionHeading({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <h2
      className={[
        "text-heading-sm m-0 text-ink",
        className,
      ].join(" ")}
    >
      {children}
    </h2>
  );
}
