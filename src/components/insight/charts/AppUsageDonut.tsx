import type { AppUsageEntry } from "../../../hooks/useInsights";
import { useUiLocale } from "../../../i18n/useUiLocale";
import { APP_SEGMENT_COLORS } from "./chartTheme";

interface AppUsageDonutProps {
  data: AppUsageEntry[];
}

const SIZE = 160;
const STROKE = 22;
const RADIUS = (SIZE - STROKE) / 2;
const CENTER = SIZE / 2;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

export function AppUsageDonut({ data }: AppUsageDonutProps) {
  const { t, formatNumber } = useUiLocale();
  const totalWords = data.reduce((sum, entry) => sum + entry.wordCount, 0);
  let offset = 0;

  const segments =
    totalWords > 0
      ? data.map((entry, index) => {
          const fraction = entry.wordCount / totalWords;
          const length = fraction * CIRCUMFERENCE;
          const segment = {
            entry,
            color: APP_SEGMENT_COLORS[index % APP_SEGMENT_COLORS.length],
            dashArray: `${length} ${CIRCUMFERENCE - length}`,
            dashOffset: -offset,
          };
          offset += length;
          return segment;
        })
      : [];

  return (
    <div className="flex flex-col items-center gap-6 sm:flex-row sm:items-start">
      <figure className="relative m-0 shrink-0">
        <svg
          width={SIZE}
          height={SIZE}
          viewBox={`0 0 ${SIZE} ${SIZE}`}
          role="img"
          aria-label={t("insight.charts.appUsage.aria")}
          className="-rotate-90"
        >
          <circle
            cx={CENTER}
            cy={CENTER}
            r={RADIUS}
            fill="none"
            stroke="var(--color-surface-deep)"
            strokeWidth={STROKE}
          />
          {segments.map((segment) => (
            <circle
              key={segment.entry.exeName}
              cx={CENTER}
              cy={CENTER}
              r={RADIUS}
              fill="none"
              stroke={segment.color}
              strokeWidth={STROKE}
              strokeDasharray={segment.dashArray}
              strokeDashoffset={segment.dashOffset}
              strokeLinecap="butt"
              opacity={0.9}
            />
          ))}
        </svg>
        <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center text-center">
          <span className="text-heading-sm text-ink">
            {formatNumber(totalWords)}
          </span>
          <span className="text-caption text-ash">
            {t("insight.charts.appUsage.centerWords")}
          </span>
        </div>
      </figure>

      <ul className="m-0 flex min-w-0 flex-1 list-none flex-col gap-3 p-0">
        {data.map((entry, index) => {
          const percent =
            totalWords > 0
              ? Math.round((entry.wordCount / totalWords) * 100)
              : 0;
          const color = APP_SEGMENT_COLORS[index % APP_SEGMENT_COLORS.length];
          return (
            <li key={entry.exeName} className="space-y-1.5">
              <div className="flex items-baseline justify-between gap-3">
                <div className="flex min-w-0 items-center gap-2">
                  <span
                    className="inline-block size-2 shrink-0 rounded-full"
                    style={{ backgroundColor: color }}
                  />
                  <span className="truncate text-body-sm text-ink">
                    {entry.exeName}
                  </span>
                </div>
                <span className="shrink-0 text-caption text-ash">
                  {t("common.percent", { value: percent })}
                </span>
              </div>
              <div className="h-1.5 overflow-hidden rounded-full border border-hairline bg-surface-deep">
                <div
                  className="h-full rounded-full"
                  style={{
                    width: `${Math.max(percent, entry.wordCount > 0 ? 4 : 0)}%`,
                    backgroundColor: color,
                    opacity: 0.75,
                  }}
                />
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
