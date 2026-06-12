import { Check, RotateCcw } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useUiLocale } from "../../i18n/useUiLocale";
import type { DictationEntry } from "../../hooks/useHistory";
import { BadgePill } from "../ui/BadgePill";
import { CopyButton } from "../ui/CopyButton";
import {
  formatAppLabel,
  formatEntryClock,
  formatEntryTime,
} from "./historyUtils";

interface HistoryEntryRowProps {
  entry: DictationEntry;
  busy: boolean;
  feedback?: "copied" | "injected";
  onCopy: (id: number) => void;
  onReinject: (id: number) => void;
}

export function HistoryEntryRow({
  entry,
  busy,
  feedback,
  onCopy,
  onReinject,
}: HistoryEntryRowProps) {
  const { t } = useTranslation();
  const { intlLocale } = useUiLocale();
  const appLabel = formatAppLabel(entry);

  return (
    <article
      className={[
        "group rounded-lg border border-hairline-strong bg-surface-card",
        "transition-colors duration-150 hover:bg-surface-elevated/40",
      ].join(" ")}
    >
      <div className="flex flex-col gap-3 p-4 sm:flex-row sm:items-start sm:gap-4 sm:p-5">
        <div className="hidden shrink-0 sm:block">
          <time
            dateTime={entry.created_at}
            className="text-caption tabular-nums text-ash"
          >
            {formatEntryClock(entry.created_at, intlLocale)}
          </time>
        </div>

        <div className="min-w-0 flex-1">
          <p className="text-body-md m-0 line-clamp-3 whitespace-pre-wrap text-ink">
            {entry.text}
          </p>
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <time
              dateTime={entry.created_at}
              className="text-caption text-ash sm:hidden"
            >
              {formatEntryTime(entry.created_at, intlLocale)}
            </time>
            <span className="hidden text-caption text-ash sm:inline">
              {formatEntryTime(entry.created_at, intlLocale)}
            </span>
            {appLabel && (
              <BadgePill className="max-w-[200px] truncate">{appLabel}</BadgePill>
            )}
            <BadgePill>
              {t("history.entry.wordCount", { count: entry.wordCount })}
            </BadgePill>
            {entry.totalMs > 0 && (
              <span className="text-caption text-ash">{entry.totalMs} ms</span>
            )}
            {feedback === "injected" && (
              <span className="inline-flex items-center gap-1 text-caption text-accent-green">
                <Check size={12} aria-hidden />
                {t("common.reinjected")}
              </span>
            )}
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-1 sm:opacity-0 sm:transition-opacity sm:group-hover:opacity-100 sm:group-focus-within:opacity-100">
          <CopyButton
            copied={feedback === "copied"}
            label={t("history.entry.copy")}
            copiedLabel={t("common.copied")}
            disabled={busy}
            onClick={() => onCopy(entry.id)}
          />
          <ActionButton
            label={t("history.entry.reinject")}
            disabled={busy}
            onClick={() => onReinject(entry.id)}
          >
            <RotateCcw size={15} strokeWidth={1.75} />
          </ActionButton>
        </div>
      </div>
    </article>
  );
}

function ActionButton({
  label,
  disabled,
  onClick,
  children,
}: {
  label: string;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      className={[
        "inline-flex size-8 items-center justify-center rounded-md border border-transparent",
        "text-charcoal transition-colors duration-150",
        "hover:border-hairline-strong hover:bg-surface-elevated hover:text-ink",
        "disabled:cursor-not-allowed disabled:opacity-40",
      ].join(" ")}
    >
      {children}
    </button>
  );
}
