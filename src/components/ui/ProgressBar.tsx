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
  const percent = Math.min(100, Math.max(0, (value / max) * 100));

  return (
    <div className={["flex flex-col gap-2", className].join(" ")}>
      {label && (
        <div className="flex items-center justify-between text-caption text-charcoal">
          <span>{label}</span>
          <span>{Math.round(percent)} %</span>
        </div>
      )}
      <div
        className="h-1.5 w-full overflow-hidden rounded-full border border-hairline bg-surface-deep"
        role="progressbar"
        aria-valuenow={Math.round(percent)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={label}
      >
        <div
          className="h-full rounded-full bg-accent-blue transition-[width] duration-300 ease-out"
          style={{ width: `${percent}%` }}
        />
      </div>
    </div>
  );
}
