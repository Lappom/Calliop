import type { ToneProfile } from "../../hooks/useAppContext";
import { TONE_META } from "./styleUtils";

const TONE_BADGE_STYLES: Record<
  ToneProfile,
  { border: string; background: string; text: string }
> = {
  default: {
    border: "rgba(255, 255, 255, 0.14)",
    background: "rgba(255, 255, 255, 0.06)",
    text: "var(--color-charcoal)",
  },
  casual: {
    border: "rgba(17, 255, 153, 0.35)",
    background: "rgba(34, 255, 153, 0.12)",
    text: "var(--color-accent-green)",
  },
  formal: {
    border: "rgba(59, 158, 255, 0.35)",
    background: "rgba(0, 117, 255, 0.14)",
    text: "var(--color-accent-blue)",
  },
  technical: {
    border: "rgba(255, 128, 31, 0.35)",
    background: "rgba(255, 89, 0, 0.14)",
    text: "var(--color-accent-orange)",
  },
};

interface ToneBadgeProps {
  tone: ToneProfile;
  className?: string;
}

export function ToneBadge({ tone, className = "" }: ToneBadgeProps) {
  const styles = TONE_BADGE_STYLES[tone];
  const label = TONE_META[tone].label;

  return (
    <span
      className={[
        "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1",
        "font-[family-name:var(--font-ui)] text-xs font-medium leading-normal",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      style={{
        borderColor: styles.border,
        backgroundColor: styles.background,
        color: styles.text,
      }}
    >
      <span
        className="size-1.5 shrink-0 rounded-full"
        style={{ backgroundColor: TONE_META[tone].accent }}
        aria-hidden
      />
      {label}
    </span>
  );
}
