import { Check, Copy } from "lucide-react";
import { IconButton } from "./IconButton";

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
    <IconButton
      label={copied && copiedLabel ? copiedLabel : label}
      disabled={disabled}
      className={className}
      onClick={onClick}
    >
      <span className="copy-icon-swap" data-copied={copied ? "true" : "false"}>
        <span data-icon="copy" aria-hidden>
          <Copy size={15} strokeWidth={1.75} />
        </span>
        <span data-icon="check" aria-hidden>
          <Check size={15} strokeWidth={1.75} className="text-accent-green" />
        </span>
      </span>
    </IconButton>
  );
}
