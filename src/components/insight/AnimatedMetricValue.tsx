import { useSmoothProgress } from "../../lib/motion/useSmoothProgress";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";

interface AnimatedMetricValueProps {
  /** Numeric target for count-up; non-numeric strings render instantly. */
  value: number | string;
  format?: (value: number) => string;
  className?: string;
}

export function AnimatedMetricValue({
  value,
  format,
  className = "",
}: AnimatedMetricValueProps) {
  const reducedMotion = useReducedMotion();
  const isNumeric = typeof value === "number";
  const numericTarget = isNumeric ? value : 0;
  const display = useSmoothProgress(
    numericTarget,
    reducedMotion || !isNumeric,
  );

  if (!isNumeric) {
    return <span className={className}>{value}</span>;
  }

  const formatted = format ? format(display) : String(Math.round(display));

  return <span className={className}>{formatted}</span>;
}
