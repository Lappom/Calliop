import { RefreshCw } from "lucide-react";

interface RefreshIconProps {
  spinning?: boolean;
  size?: number;
  className?: string;
}

export function RefreshIcon({
  spinning = false,
  size = 16,
  className = "",
}: RefreshIconProps) {
  return (
    <RefreshCw
      size={size}
      strokeWidth={1.75}
      aria-hidden
      className={[spinning ? "animate-spin" : "", className].filter(Boolean).join(" ")}
    />
  );
}
