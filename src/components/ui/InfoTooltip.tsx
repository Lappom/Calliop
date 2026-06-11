import { useId, type ReactNode } from "react";
import { CircleHelp } from "lucide-react";

interface InfoTooltipProps {
  content: string;
  children?: ReactNode;
  className?: string;
}

export function InfoTooltip({
  content,
  children,
  className = "",
}: InfoTooltipProps) {
  const tooltipId = useId();

  return (
    <span
      className={[
        "group/info relative inline-flex shrink-0 align-middle",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <span
        aria-describedby={tooltipId}
        className="inline-flex cursor-default text-ash transition-colors hover:text-charcoal"
      >
        {children ?? <CircleHelp size={15} strokeWidth={1.75} aria-hidden />}
        <span className="sr-only">{content}</span>
      </span>
      <span
        id={tooltipId}
        role="tooltip"
        className={[
          "pointer-events-none absolute left-1/2 top-full z-50 mt-2 w-56 -translate-x-1/2",
          "rounded-md border border-hairline-strong bg-surface-elevated px-3 py-2",
          "text-caption leading-snug text-charcoal",
          "opacity-0 transition-opacity duration-150",
          "group-hover/info:opacity-100",
        ].join(" ")}
      >
        {content}
      </span>
    </span>
  );
}
