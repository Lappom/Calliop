type StatusColor = "green" | "blue" | "red" | "yellow" | "mute";

interface StatusDotProps {
  color?: StatusColor;
  className?: string;
  label?: string;
}

const colorClasses: Record<StatusColor, string> = {
  green: "bg-accent-green",
  blue: "bg-accent-blue",
  red: "bg-accent-red",
  yellow: "bg-accent-yellow",
  mute: "bg-stone",
};

export function StatusDot({
  color = "green",
  className = "",
  label,
}: StatusDotProps) {
  return (
    <span
      className={["inline-flex items-center gap-2", className].join(" ")}
      role={label ? "status" : undefined}
      aria-label={label}
    >
      <span
        className={[
          "size-2 shrink-0 rounded-full",
          colorClasses[color],
        ].join(" ")}
        aria-hidden="true"
      />
      {label && (
        <span className="text-body-sm text-body">{label}</span>
      )}
    </span>
  );
}
