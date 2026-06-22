import type { ButtonHTMLAttributes, ReactNode } from "react";

type IconButtonSize = "sm" | "md";
type IconButtonTone = "default" | "danger";

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  size?: IconButtonSize;
  tone?: IconButtonTone;
  active?: boolean;
  children: ReactNode;
}

const sizeClasses: Record<IconButtonSize, string> = {
  sm: "size-8",
  md: "size-9",
};

export function IconButton({
  label,
  size = "sm",
  tone = "default",
  active = false,
  className = "",
  children,
  type = "button",
  ...props
}: IconButtonProps) {
  return (
    <button
      type={type}
      aria-label={label}
      className={[
        "inline-flex shrink-0 items-center justify-center rounded-md border",
        sizeClasses[size],
        "transition-[transform,background-color,border-color,color] duration-150",
        "motion-reduce:transition-none",
        "ease-[cubic-bezier(0.22,1,0.36,1)]",
        "active:scale-[0.97] motion-reduce:active:scale-100 disabled:active:scale-100",
        "disabled:cursor-not-allowed disabled:opacity-40",
        active
          ? "border-hairline-strong bg-surface-elevated text-ink"
          : tone === "danger"
            ? "border-transparent text-charcoal hover:border-hairline-strong hover:bg-surface-elevated hover:text-accent-red"
            : "border-transparent text-charcoal hover:border-hairline-strong hover:bg-surface-elevated hover:text-ink",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      {...props}
    >
      {children}
    </button>
  );
}
