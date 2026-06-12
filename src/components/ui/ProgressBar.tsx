import { useSmoothProgress } from "../../lib/motion/useSmoothProgress";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

interface ProgressBarProps {
  value: number;
  max?: number;
  label?: string;
  className?: string;
}

export function ProgressBar({
  value,
  max = 100,
  label,
  className = "",
}: ProgressBarProps) {
  const reducedMotion = useReducedMotion();
  const percent = Math.min(100, Math.max(0, (value / max) * 100));
  const smoothPercent = useSmoothProgress(percent, reducedMotion);
  const displayPercent = Math.round(smoothPercent);
  const targetPercent = Math.round(percent);

  return (
    <div className={["flex flex-col gap-2", className].join(" ")}>
      {label && (
        <div className="flex items-center justify-between text-caption text-charcoal">
          <span>{label}</span>
          <span>{displayPercent} %</span>
        </div>
      )}
      <div
        className="h-1.5 w-full overflow-hidden rounded-full border border-hairline bg-surface-deep"
        role="progressbar"
        aria-valuenow={targetPercent}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={label}
      >
        <div
          className="h-full w-full origin-left rounded-full bg-accent-blue"
          style={{ transform: `scaleX(${smoothPercent / 100})` }}
        />
      </div>
    </div>
  );
}
