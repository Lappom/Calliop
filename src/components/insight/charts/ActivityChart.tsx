import type { DailyActivityEntry } from "../../../hooks/useInsights";
import { ChartFrame } from "./ChartFrame";
import { CHART_COLORS, formatShortDate } from "./chartTheme";
import {
  CHART_PADDING,
  CHART_VIEW_HEIGHT,
  CHART_VIEW_WIDTH,
  computeBarLayout,
  gridLineRatios,
  plotDimensions,
} from "./chartLayout";

interface ActivityChartProps {
  data: DailyActivityEntry[];
}

export function ActivityChart({ data }: ActivityChartProps) {
  const maxWords = Math.max(...data.map((d) => d.wordCount), 1);
  const { plotWidth, plotHeight } = plotDimensions();
  const { barWidth, gap, groupOffset } = computeBarLayout(
    data.length,
    plotWidth,
    { maxBarWidth: 26, gap: 6 },
  );

  const gridLines = gridLineRatios(4).map((ratio) => ({
    y: CHART_PADDING.top + plotHeight * (1 - ratio),
    label: Math.round(maxWords * ratio),
  }));

  return (
    <ChartFrame ariaLabel="Graphique des mots dictés sur les sept derniers jours">
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

        {data.map((entry, index) => {
          const barHeight =
            maxWords > 0 ? (entry.wordCount / maxWords) * plotHeight : 0;
          const x =
            CHART_PADDING.left +
            groupOffset +
            index * (barWidth + gap);
          const y = CHART_PADDING.top + plotHeight - barHeight;

          return (
            <g key={entry.date}>
              <rect
                x={x}
                y={y}
                width={barWidth}
                height={Math.max(barHeight, entry.wordCount > 0 ? 3 : 0)}
                rx={3}
                fill={CHART_COLORS.blue}
                fillOpacity={entry.wordCount > 0 ? 0.85 : 0.12}
              />
              <text
                x={x + barWidth / 2}
                y={CHART_VIEW_HEIGHT - 12}
                textAnchor="middle"
                className="fill-charcoal font-[family-name:var(--font-ui)] text-[10px]"
              >
                {formatShortDate(entry.date)}
              </text>
            </g>
          );
        })}
      </svg>
    </ChartFrame>
  );
}
