import { Check, Copy } from "lucide-react";

interface CopyButtonProps {
  copied: boolean;
  label: string;
  copiedLabel?: string;
  disabled?: boolean;
  className?: string;
  onClick: () => void;
}

export function CopyButton({
  copied,
  label,
  copiedLabel,
  disabled,
  className = "",
  onClick,
}: CopyButtonProps) {
  return (
    <button
      type="button"
      aria-label={copied && copiedLabel ? copiedLabel : label}
      disabled={disabled}
      onClick={onClick}
      className={[
        "inline-flex size-8 shrink-0 items-center justify-center rounded-md border border-transparent",
        "text-charcoal transition-[transform,background-color,border-color,color]",
        "duration-[var(--motion-fast)] ease-[var(--ease-enter)]",
        "hover:border-hairline-strong hover:bg-surface-elevated hover:text-ink",
        "active:scale-[0.97] disabled:active:scale-100",
        "disabled:cursor-not-allowed disabled:opacity-40",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <span className="copy-icon-swap" data-copied={copied ? "true" : "false"}>
        <span data-icon="copy" aria-hidden>
          <Copy size={15} strokeWidth={1.75} />
        </span>
        <span data-icon="check" aria-hidden>
          <Check size={15} strokeWidth={1.75} className="text-accent-green" />
        </span>
      </span>
    </button>
  );
}
