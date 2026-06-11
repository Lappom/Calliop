import type { ReactNode } from "react";

export function SnippetListToolbarButton({
  label,
  active = false,
  disabled,
  onClick,
  children,
}: {
  label: string;
  active?: boolean;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      aria-pressed={active}
      disabled={disabled}
      onClick={onClick}
      className={[
        "inline-flex size-9 items-center justify-center rounded-md border transition-colors duration-150",
        "disabled:cursor-not-allowed disabled:opacity-40",
        active
          ? "border-hairline-strong bg-surface-elevated text-ink"
          : "border-transparent text-charcoal hover:border-hairline-strong hover:bg-surface-elevated hover:text-ink",
      ].join(" ")}
    >
      {children}
    </button>
  );
}
