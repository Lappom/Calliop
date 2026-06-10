import type { HTMLAttributes, ReactNode } from "react";

type CardVariant = "default" | "bordered" | "elevated" | "deep";

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  variant?: CardVariant;
  children: ReactNode;
}

const variantClasses: Record<CardVariant, string> = {
  default: "bg-surface-card",
  bordered: "bg-surface-card border border-hairline-strong",
  elevated: "bg-surface-elevated border border-hairline-strong",
  deep: "bg-surface-deep border border-hairline-strong",
};

export function Card({
  variant = "default",
  className = "",
  children,
  ...props
}: CardProps) {
  return (
    <div
      className={[
        "rounded-lg p-8 text-ink",
        variantClasses[variant],
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      {...props}
    >
      {children}
    </div>
  );
}
