import type { HTMLAttributes, ReactNode } from "react";

interface KbdProps extends HTMLAttributes<HTMLElement> {
  children: ReactNode;
}

export function Kbd({ children, className = "", ...props }: KbdProps) {
  return (
    <kbd
      className={[
        "inline-flex min-w-[1.5rem] items-center justify-center",
        "rounded-xs border border-hairline-strong bg-surface-card",
        "px-1.5 py-0.5 text-code-md text-ink",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      {...props}
    >
      {children}
    </kbd>
  );
}
