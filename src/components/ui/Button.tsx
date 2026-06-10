import type { ButtonHTMLAttributes, ReactNode } from "react";

type ButtonVariant = "primary" | "ghost" | "outline";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  children: ReactNode;
}

const variantClasses: Record<ButtonVariant, string> = {
  primary:
    "bg-primary text-primary-on border border-transparent hover:bg-surface-light active:bg-surface-light",
  ghost:
    "bg-surface-elevated text-ink border border-hairline-strong hover:border-ink/30",
  outline:
    "bg-canvas text-ink border border-hairline-strong hover:border-ink/30",
};

export function Button({
  variant = "primary",
  className = "",
  children,
  type = "button",
  ...props
}: ButtonProps) {
  return (
    <button
      type={type}
      className={[
        "inline-flex h-9 items-center justify-center rounded-md px-4",
        "font-[family-name:var(--font-ui)] text-sm font-medium leading-[1.43]",
        "transition-colors duration-150",
        "disabled:cursor-not-allowed disabled:opacity-40",
        variantClasses[variant],
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
