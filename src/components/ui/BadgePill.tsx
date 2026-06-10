import type { HTMLAttributes, ReactNode } from "react";

interface BadgePillProps extends HTMLAttributes<HTMLSpanElement> {
  children: ReactNode;
  active?: boolean;
}

export function BadgePill({
  children,
  active = false,
  className = "",
  ...props
}: BadgePillProps) {
  return (
    <span
      className={[
        "inline-flex items-center rounded-full px-2.5 py-1",
        "font-[family-name:var(--font-ui)] text-xs leading-normal",
        active
          ? "bg-surface-elevated text-ink border border-hairline-strong"
          : "bg-surface-elevated text-body",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      {...props}
    >
      {children}
    </span>
  );
}
