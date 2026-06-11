import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { DictionarySource } from "../../hooks/useDictionary";
import { getSourceMeta } from "./dictionaryUtils";

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
  const { t } = useTranslation();
  const sourceMeta = useMemo(() => getSourceMeta(t), [t]);
  const styles = SOURCE_BADGE_STYLES[source];
  const label = sourceMeta[source].label;

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
        style={{ backgroundColor: sourceMeta[source].accent }}
        aria-hidden
      />
      {label}
    </span>
  );
}
