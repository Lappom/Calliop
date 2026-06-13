import { Fragment, useMemo } from "react";
import type { HourAppHeatmapCell } from "../../../hooks/useInsights";
import { useUiLocale } from "../../../i18n/useUiLocale";
import { useReducedMotion } from "../../../lib/motion/useReducedMotion";
import { buildHourAppHeatmap } from "../insightUtils";
import { CHART_COLORS } from "./chartTheme";

interface HourAppHeatmapChartProps {
  data: HourAppHeatmapCell[];
}

const HOURS = Array.from({ length: 24 }, (_, hour) => hour);

export function HourAppHeatmapChart({ data }: HourAppHeatmapChartProps) {
  const { t, formatNumber, intlLocale } = useUiLocale();
  const reducedMotion = useReducedMotion();
  const matrix = useMemo(() => buildHourAppHeatmap(data), [data]);

  if (matrix.apps.length === 0) {
    return null;
  }

  const hourFormatter = new Intl.DateTimeFormat(intlLocale, {
    hour: "numeric",
  });

  function formatHourLabel(hour: number): string {
    const date = new Date(2000, 0, 1, hour, 0, 0);
    return hourFormatter.format(date);
  }

  return (
    <div className="w-full overflow-x-auto">
      <div
        className="min-w-[640px]"
        role="grid"
        aria-label={t("insight.charts.heatmap.aria")}
      >
        <div
          className="grid gap-1"
          style={{
            gridTemplateColumns: `minmax(88px, 1.2fr) repeat(24, minmax(16px, 1fr))`,
          }}
        >
          <div role="rowheader" className="text-caption text-charcoal" />
          {HOURS.map((hour) => (
            <div
              key={`h-${hour}`}
              role="columnheader"
              className="text-caption text-center text-ash"
            >
              {hour % 3 === 0 ? formatHourLabel(hour) : ""}
            </div>
          ))}

          {matrix.apps.map((app, rowIndex) => (
            <Fragment key={app}>
              <div
                role="rowheader"
                className="truncate pr-2 text-body-sm text-ink"
                title={app}
              >
                {app}
              </div>
              {HOURS.map((hour, colIndex) => {
                const value = matrix.lookup.get(`${app}:${hour}`) ?? 0;
                const intensity =
                  matrix.max > 0 && value > 0 ? value / matrix.max : 0;
                const delayMs = reducedMotion
                  ? 0
                  : (rowIndex * 24 + colIndex) * 8;

                return (
                  <div
                    key={`${app}-${hour}`}
                    role="gridcell"
                    className="insight-heatmap-cell aspect-square min-h-4 rounded-xs border border-hairline"
                    style={{
                      backgroundColor:
                        intensity > 0
                          ? `color-mix(in srgb, ${CHART_COLORS.blue} ${Math.round(12 + intensity * 68)}%, transparent)`
                          : "var(--color-surface-deep)",
                      transitionDelay: `${delayMs}ms`,
                    }}
                    title={
                      value > 0
                        ? t("insight.charts.heatmap.cellTooltip", {
                            app,
                            hour: formatHourLabel(hour),
                            count: formatNumber(value),
                          })
                        : undefined
                    }
                    aria-label={
                      value > 0
                        ? t("insight.charts.heatmap.cellAria", {
                            app,
                            hour: formatHourLabel(hour),
                            count: formatNumber(value),
                          })
                        : t("insight.charts.heatmap.cellEmpty", {
                            app,
                            hour: formatHourLabel(hour),
                          })
                    }
                  />
                );
              })}
            </Fragment>
          ))}
        </div>
      </div>
    </div>
  );
}
