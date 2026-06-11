import type { DictionarySource } from "../../hooks/useDictionary";
import { SOURCE_META } from "./dictionaryUtils";

const SOURCE_BADGE_STYLES: Record<
  DictionarySource,
  { border: string; background: string; text: string }
> = {
  manual: {
    border: "rgba(59, 158, 255, 0.35)",
    background: "rgba(0, 117, 255, 0.14)",
    text: "var(--color-accent-blue)",
  },
  learned: {
    border: "rgba(17, 255, 153, 0.35)",
    background: "rgba(34, 255, 153, 0.12)",
    text: "var(--color-accent-green)",
  },
};

interface DictionarySourceBadgeProps {
  source: DictionarySource;
  className?: string;
}

export function DictionarySourceBadge({
  source,
  className = "",
}: DictionarySourceBadgeProps) {
  const styles = SOURCE_BADGE_STYLES[source];
  const label = SOURCE_META[source].label;

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
        style={{ backgroundColor: SOURCE_META[source].accent }}
        aria-hidden
      />
      {label}
    </span>
  );
}
