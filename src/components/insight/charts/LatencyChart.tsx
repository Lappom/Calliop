import { useEffect, useMemo, useState } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  type TooltipContentProps,
  XAxis,
  YAxis,
} from "recharts";
import type { RecentLatencyEntry } from "../../../hooks/useInsights";
import { useUiLocale } from "../../../i18n/useUiLocale";
import { useReducedMotion } from "../../../lib/motion/useReducedMotion";
import { ChartFrame } from "./ChartFrame";
import { CHART_COLORS, formatLatencyAxisTime } from "./chartTheme";
import {
  rechartsTooltipStyle,
  RECHARTS_AXIS_TICK,
  RECHARTS_MARGIN,
} from "./rechartsTheme";

interface LatencyChartProps {
  data: RecentLatencyEntry[];
}

interface LatencyChartRow {
  id: string;
  label: string;
  sttMs: number;
  llmMs: number;
  injectMs: number;
  totalMs: number;
}

const CHART_ANIMATION_MS = 400;

function LatencyTooltip(props: TooltipContentProps) {
  const { active, payload, label } = props;
  if (!active || !payload?.length) {
    return null;
  }

  const rows = payload.filter(
    (item) => typeof item.value === "number" && item.value > 0,
  );

  return (
    <div style={rechartsTooltipStyle}>
      <p className="m-0 mb-2 text-caption text-ink">{label}</p>
      <ul className="m-0 list-none space-y-1 p-0">
        {rows.map((item) => (
          <li
            key={String(item.dataKey)}
            className="flex items-center justify-between gap-4 text-caption"
          >
            <span className="flex items-center gap-2">
              <span
                className="inline-block size-2 rounded-full"
                style={{ backgroundColor: item.color }}
              />
              {item.name}
            </span>
            <span className="font-[family-name:var(--font-mono)] tabular-nums text-ink">
              {item.value} ms
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

export function LatencyChart({ data }: LatencyChartProps) {
  const { t, intlLocale } = useUiLocale();
  const reducedMotion = useReducedMotion();
  const [animate, setAnimate] = useState(!reducedMotion);

  useEffect(() => {
    if (reducedMotion) {
      setAnimate(false);
      return;
    }
    setAnimate(true);
  }, [data, reducedMotion]);

  const segments = useMemo(
    () => [
      {
        key: "sttMs" as const,
        label: t("insight.charts.latency.stt"),
        color: CHART_COLORS.blue,
      },
      {
        key: "llmMs" as const,
        label: t("insight.charts.latency.llm"),
        color: CHART_COLORS.orange,
      },
      {
        key: "injectMs" as const,
        label: t("insight.charts.latency.inject"),
        color: CHART_COLORS.green,
      },
    ],
    [t],
  );

  const chartData = useMemo(
    (): LatencyChartRow[] =>
      data.map((entry, index) => ({
        id: `${entry.created_at}-${index}`,
        label: formatLatencyAxisTime(entry.created_at, intlLocale),
        sttMs: entry.sttMs,
        llmMs: entry.llmMs,
        injectMs: entry.injectMs,
        totalMs: entry.totalMs,
      })),
    [data, intlLocale],
  );

  const barAnimation = {
    isAnimationActive: animate,
    animationDuration: CHART_ANIMATION_MS,
    animationEasing: "ease-out" as const,
  };

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
    <ChartFrame ariaLabel={t("insight.charts.latency.aria")} legend={legend}>
      <ResponsiveContainer width="100%" height={200}>
        <BarChart data={chartData} margin={RECHARTS_MARGIN} barCategoryGap="20%">
          <CartesianGrid
            stroke={CHART_COLORS.grid}
            vertical={false}
            strokeDasharray="0"
          />
          <XAxis
            dataKey="label"
            tick={RECHARTS_AXIS_TICK}
            tickLine={false}
            axisLine={{ stroke: CHART_COLORS.hairline }}
            interval={0}
            tickFormatter={(value, index) => {
              if (index === 0) return value;
              return value === chartData[index - 1]?.label ? "" : value;
            }}
          />
          <YAxis
            tick={RECHARTS_AXIS_TICK}
            tickLine={false}
            axisLine={false}
            width={36}
            tickFormatter={(value: number) => `${value}`}
          />
          <Tooltip
            content={LatencyTooltip}
            cursor={{ fill: "rgba(255, 255, 255, 0.04)" }}
          />
          <Bar
            dataKey="sttMs"
            name={segments[0].label}
            stackId="latency"
            fill={CHART_COLORS.blue}
            fillOpacity={0.9}
            maxBarSize={28}
            radius={[0, 0, 3, 3]}
            {...barAnimation}
          />
          <Bar
            dataKey="llmMs"
            name={segments[1].label}
            stackId="latency"
            fill={CHART_COLORS.orange}
            fillOpacity={0.9}
            maxBarSize={28}
            {...barAnimation}
          />
          <Bar
            dataKey="injectMs"
            name={segments[2].label}
            stackId="latency"
            fill={CHART_COLORS.green}
            fillOpacity={0.9}
            maxBarSize={28}
            radius={[3, 3, 0, 0]}
            {...barAnimation}
          />
        </BarChart>
      </ResponsiveContainer>
    </ChartFrame>
  );
}
