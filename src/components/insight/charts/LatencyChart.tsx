import { useMemo } from "react";
import type { RecentLatencyEntry } from "../../../hooks/useInsights";
import { useUiLocale } from "../../../i18n/useUiLocale";
import { ChartFrame } from "./ChartFrame";
import { CHART_COLORS, formatShortTime } from "./chartTheme";
import {
  CHART_PADDING,
  CHART_VIEW_HEIGHT,
  CHART_VIEW_WIDTH,
  computeBarLayout,
  gridLineRatios,
  plotDimensions,
} from "./chartLayout";

interface LatencyChartProps {
  data: RecentLatencyEntry[];
}

export function LatencyChart({ data }: LatencyChartProps) {
  const { t, intlLocale } = useUiLocale();

  const segments = useMemo(
    () => [
      { key: "sttMs" as const, label: t("insight.charts.latency.stt"), color: CHART_COLORS.blue },
      { key: "llmMs" as const, label: t("insight.charts.latency.llm"), color: CHART_COLORS.orange },
      {
        key: "injectMs" as const,
        label: t("insight.charts.latency.inject"),
        color: CHART_COLORS.green,
      },
    ],
    [t],
  );

  const maxTotal = Math.max(...data.map((d) => d.totalMs), 1);
  const { plotWidth, plotHeight } = plotDimensions();
  const { barWidth, gap, groupOffset } = computeBarLayout(data.length, plotWidth, {
    maxBarWidth: 32,
    gap: 10,
  });

  const gridLines = gridLineRatios(4).map((ratio) => ({
    y: CHART_PADDING.top + plotHeight * (1 - ratio),
    label: `${Math.round(maxTotal * ratio)}`,
  }));

  const legend = (
    <ul className="m-0 flex list-none flex-wrap gap-4 p-0">
      {segments.map((segment) => (
        <li
          key={segment.key}
          className="flex items-center gap-2 text-caption text-charcoal"
        >
          <span
            className="inline-block size-2 rounded-full"
            style={{ backgroundColor: segment.color }}
          />
          {segment.label}
        </li>
      ))}
    </ul>
  );

  return (
    <ChartFrame
      ariaLabel={t("insight.charts.latency.aria")}
      legend={legend}
    >
      <svg
        viewBox={`0 0 ${CHART_VIEW_WIDTH} ${CHART_VIEW_HEIGHT}`}
        preserveAspectRatio="xMidYMid meet"
        className="h-full w-full"
        role="img"
      >
        {gridLines.map((line) => (
          <g key={line.y}>
            <line
              x1={CHART_PADDING.left}
              y1={line.y}
              x2={CHART_VIEW_WIDTH - CHART_PADDING.right}
              y2={line.y}
              stroke={CHART_COLORS.grid}
              strokeWidth={1}
            />
            <text
              x={CHART_PADDING.left - 8}
              y={line.y + 4}
              textAnchor="end"
              className="fill-ash font-[family-name:var(--font-mono)] text-[10px]"
            >
              {line.label}
            </text>
          </g>
        ))}

        <line
          x1={CHART_PADDING.left}
          y1={CHART_PADDING.top + plotHeight}
          x2={CHART_VIEW_WIDTH - CHART_PADDING.right}
          y2={CHART_PADDING.top + plotHeight}
          stroke={CHART_COLORS.hairline}
          strokeWidth={1}
        />

        {data.map((entry, index) => {
          const x =
            CHART_PADDING.left +
            groupOffset +
            index * (barWidth + gap);
          let stackY = CHART_PADDING.top + plotHeight;

          return (
            <g key={`${entry.created_at}-${index}`}>
              {segments.map((segment) => {
                const value = entry[segment.key];
                const height = (value / maxTotal) * plotHeight;
                stackY -= height;
                if (value <= 0) {
                  return null;
                }
                return (
                  <rect
                    key={segment.key}
                    x={x}
                    y={stackY}
                    width={barWidth}
                    height={Math.max(height, 1)}
                    rx={2}
                    fill={segment.color}
                    fillOpacity={0.9}
                  />
                );
              })}
              <text
                x={x + barWidth / 2}
                y={CHART_VIEW_HEIGHT - 12}
                textAnchor="middle"
                className="fill-charcoal font-[family-name:var(--font-mono)] text-[10px]"
              >
                {formatShortTime(entry.created_at, intlLocale)}
              </text>
            </g>
          );
        })}
      </svg>
    </ChartFrame>
  );
}
