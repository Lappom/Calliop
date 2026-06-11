import { useUiLocale } from "../../../i18n/useUiLocale";
import { CHART_COLORS } from "./chartTheme";

interface WpmGaugeProps {
  percent: number;
  averageWpm: number;
  baselineWpm?: number;
}

const WIDTH = 160;
const STROKE = 10;
const RADIUS = 50;
const PAD = STROKE / 2 + 4;
const CX = WIDTH / 2;
const BASELINE_Y = PAD + RADIUS;
const HEIGHT = BASELINE_Y + PAD;
const ARC_LENGTH = Math.PI * RADIUS;

export function WpmGauge({
  percent,
  averageWpm,
  baselineWpm = 40,
}: WpmGaugeProps) {
  const { t } = useUiLocale();
  const clamped = Math.min(Math.max(percent, 0), 200);
  const progress = (clamped / 200) * ARC_LENGTH;
  const arcStartX = CX - RADIUS;
  const arcEndX = CX + RADIUS;
  const roundedWpm = Math.round(averageWpm);

  return (
    <figure
      className="m-0 flex w-full flex-col items-center gap-4"
      aria-label={t("insight.wpm.aria", {
        wpm: roundedWpm,
        percent,
      })}
    >
      <svg
        width={WIDTH}
        height={HEIGHT}
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        className="shrink-0 overflow-visible"
        role="img"
      >
        <path
          d={`M ${arcStartX} ${BASELINE_Y} A ${RADIUS} ${RADIUS} 0 0 1 ${arcEndX} ${BASELINE_Y}`}
          fill="none"
          stroke="var(--color-surface-deep)"
          strokeWidth={STROKE}
          strokeLinecap="round"
        />
        <path
          d={`M ${arcStartX} ${BASELINE_Y} A ${RADIUS} ${RADIUS} 0 0 1 ${arcEndX} ${BASELINE_Y}`}
          fill="none"
          stroke={CHART_COLORS.green}
          strokeWidth={STROKE}
          strokeLinecap="round"
          strokeDasharray={`${progress} ${ARC_LENGTH}`}
          opacity={0.9}
        />
      </svg>

      <div className="flex flex-col items-center gap-1 text-center">
        <p className="text-heading-md m-0 leading-none text-ink">
          {percent > 0 ? `${percent}%` : t("common.emDash")}
        </p>
        <p className="text-caption m-0 text-charcoal">
          {t("insight.wpm.vsBaseline", { baseline: baselineWpm })}
        </p>
      </div>
    </figure>
  );
}
