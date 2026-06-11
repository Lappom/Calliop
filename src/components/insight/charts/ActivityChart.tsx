import { useMemo } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  LabelList,
  ResponsiveContainer,
  Tooltip,
  type TooltipContentProps,
  XAxis,
  YAxis,
} from "recharts";
import type { DailyActivityEntry } from "../../../hooks/useInsights";
import { useUiLocale } from "../../../i18n/useUiLocale";
import { ChartFrame } from "./ChartFrame";
import { CHART_COLORS, formatShortDate } from "./chartTheme";
import {
  RECHARTS_AXIS_TICK,
  RECHARTS_AXIS_TICK_UI,
  RECHARTS_MARGIN,
  rechartsTooltipStyle,
} from "./rechartsTheme";

interface ActivityChartProps {
  data: DailyActivityEntry[];
}

interface ActivityChartRow {
  date: string;
  label: string;
  wordCount: number;
  dictationCount: number;
}

export function ActivityChart({ data }: ActivityChartProps) {
  const { t, intlLocale, formatNumber } = useUiLocale();

  const chartData = useMemo(
    (): ActivityChartRow[] =>
      data.map((entry) => ({
        date: entry.date,
        label: formatShortDate(entry.date, intlLocale),
        wordCount: entry.wordCount,
        dictationCount: entry.dictationCount,
      })),
    [data, intlLocale],
  );

  const wordsLabel = t("insight.week.words");

  function ActivityTooltip(props: TooltipContentProps) {
    const { active, payload, label } = props;
    if (!active || !payload?.length) {
      return null;
    }

    const row = payload[0]?.payload as ActivityChartRow | undefined;
    if (!row) {
      return null;
    }

    return (
      <div style={rechartsTooltipStyle}>
        <p className="m-0 mb-2 text-caption text-ink">{label}</p>
        <ul className="m-0 list-none space-y-1 p-0 text-caption">
          <li className="flex items-center justify-between gap-4">
            <span className="text-charcoal">{wordsLabel}</span>
            <span className="font-[family-name:var(--font-mono)] tabular-nums text-ink">
              {formatNumber(row.wordCount)}
            </span>
          </li>
          {row.dictationCount > 0 && (
            <li className="flex items-center justify-between gap-4">
              <span className="text-charcoal">{t("insight.week.dictations")}</span>
              <span className="font-[family-name:var(--font-mono)] tabular-nums text-ink">
                {row.dictationCount}
              </span>
            </li>
          )}
        </ul>
      </div>
    );
  }

  return (
    <ChartFrame ariaLabel={t("insight.charts.activity.aria")}>
      <ResponsiveContainer width="100%" height={200}>
        <BarChart data={chartData} margin={RECHARTS_MARGIN} barCategoryGap="18%">
          <CartesianGrid
            stroke={CHART_COLORS.grid}
            vertical={false}
            strokeDasharray="0"
          />
          <XAxis
            dataKey="label"
            tick={RECHARTS_AXIS_TICK_UI}
            tickLine={false}
            axisLine={{ stroke: CHART_COLORS.hairline }}
            interval={0}
          />
          <YAxis
            tick={RECHARTS_AXIS_TICK}
            tickLine={false}
            axisLine={false}
            width={36}
            allowDecimals={false}
          />
          <Tooltip
            content={ActivityTooltip}
            cursor={{ fill: "rgba(255, 255, 255, 0.04)" }}
          />
          <Bar
            dataKey="wordCount"
            name={wordsLabel}
            fill={CHART_COLORS.blue}
            maxBarSize={26}
            radius={[3, 3, 0, 0]}
            minPointSize={3}
          >
            {chartData.map((entry) => (
              <Cell
                key={entry.date}
                fillOpacity={entry.wordCount > 0 ? 0.85 : 0.12}
              />
            ))}
            <LabelList
              dataKey="dictationCount"
              position="top"
              formatter={(value) =>
                typeof value === "number" && value > 0 ? `${value}×` : ""
              }
              fill={CHART_COLORS.muted}
              fontSize={9}
              fontFamily="var(--font-mono)"
            />
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </ChartFrame>
  );
}
