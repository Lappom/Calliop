import type { ReactNode } from "react";
import type { GlowColor } from "../layout/glowSurface";
import { glowSurfaceClasses } from "../layout/glowSurface";

interface InsightChartPanelProps {
  title: string;
  description: string;
  children: ReactNode;
  empty: boolean;
  emptyMessage: string;
  glow?: GlowColor;
  className?: string;
  footer?: ReactNode;
}

export function InsightChartPanel({
  title,
  description,
  children,
  empty,
  emptyMessage,
  glow = "blue",
  className = "",
  footer,
}: InsightChartPanelProps) {
  return (
    <div
      className={[
        glowSurfaceClasses(glow),
        "flex h-full min-h-[280px] flex-col gap-4 rounded-lg border border-hairline-strong bg-surface-card p-4 sm:min-h-[360px] sm:p-6",
        className,
      ].join(" ")}
    >
      <div className="relative shrink-0">
        <h2 className="text-heading-sm m-0 text-ink">{title}</h2>
        <p className="text-body-sm mt-2 text-charcoal">{description}</p>
      </div>
      {empty ? (
        <p className="text-body-sm relative flex-1 text-charcoal">{emptyMessage}</p>
      ) : (
        <div className="relative flex flex-1 flex-col justify-end">{children}</div>
      )}
      {footer && !empty && (
        <div className="relative shrink-0 border-t border-divider-soft pt-3">
          {footer}
        </div>
      )}
    </div>
  );
}
