import type { ReactNode } from "react";
import { ArrowRight, Pencil, Trash2 } from "lucide-react";
import { useUiLocale } from "../../i18n/useUiLocale";
import type { DictionaryWord } from "../../hooks/useDictionary";
import { AnimatedTableBody } from "../motion/AnimatedTableBody";
import { DictionarySourceBadge } from "./DictionarySourceBadge";
import { formatDictionaryDate } from "./dictionaryUtils";

interface DictionaryTableProps {
  words: DictionaryWord[];
  busy: boolean;
  onEdit: (entry: DictionaryWord) => void;
  onDelete: (id: number) => void;
}

function IconActionButton({
  label,
  disabled,
  onClick,
  children,
  tone = "default",
}: {
  label: string;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
  tone?: "default" | "danger";
}) {
  return (
    <button
      type="button"
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      className={[
        "inline-flex size-8 items-center justify-center rounded-md border border-transparent",
        "transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-40",
        tone === "danger"
          ? "text-charcoal hover:border-hairline-strong hover:bg-surface-elevated hover:text-accent-red"
          : "text-charcoal hover:border-hairline-strong hover:bg-surface-elevated hover:text-ink",
      ].join(" ")}
    >
      {children}
    </button>
  );
}

export function DictionaryTable({
  words,
  busy,
  onEdit,
  onDelete,
}: DictionaryTableProps) {
  const { t, intlLocale } = useUiLocale();

  return (
    <div className="overflow-hidden rounded-lg border border-hairline-strong bg-surface-card">
      <div className="overflow-x-auto">
        <table className="w-full min-w-[480px] border-collapse">
          <thead>
            <tr className="border-b border-divider-soft text-left">
              <th className="text-caption px-4 py-3 font-medium text-ash">
                {t("dictionary.table.word")}
              </th>
              <th className="text-caption hidden px-4 py-3 font-medium text-ash sm:table-cell">
                {t("dictionary.table.source")}
              </th>
              <th className="text-caption hidden px-4 py-3 font-medium text-ash md:table-cell">
                {t("dictionary.table.addedAt")}
              </th>
              <th className="text-caption w-20 px-2 py-3 font-medium text-ash sm:w-24">
                <span className="sr-only">{t("common.actions")}</span>
              </th>
            </tr>
          </thead>
          <AnimatedTableBody
            items={words}
            getRowKey={(entry) => entry.id}
            renderRow={(entry) => (
              <>
                <td className="min-w-0 px-4 py-3.5">
                  <div className="flex min-w-0 flex-col gap-2">
                    {entry.misspelling ? (
                      <div className="flex min-w-0 flex-wrap items-center gap-2">
                        <span className="truncate font-[family-name:var(--font-body)] text-body-md text-charcoal">
                          {entry.misspelling}
                        </span>
                        <ArrowRight
                          size={14}
                          strokeWidth={1.75}
                          className="shrink-0 text-ash"
                          aria-hidden
                        />
                        <span className="truncate font-[family-name:var(--font-body)] text-body-md text-ink">
                          {entry.word}
                        </span>
                      </div>
                    ) : (
                      <span className="truncate font-[family-name:var(--font-body)] text-body-md text-ink">
                        {entry.word}
                      </span>
                    )}
                    <DictionarySourceBadge
                      source={entry.source}
                      className="sm:hidden"
                    />
                  </div>
                </td>
                <td className="hidden px-4 py-3.5 sm:table-cell">
                  <DictionarySourceBadge source={entry.source} />
                </td>
                <td className="text-body-sm hidden px-4 py-3.5 text-charcoal md:table-cell">
                  {formatDictionaryDate(entry.created_at, intlLocale)}
                </td>
                <td className="w-20 px-2 py-2 sm:w-24">
                  <div className="flex items-center justify-end gap-0.5 opacity-100 transition-opacity sm:opacity-0 sm:group-hover:opacity-100 sm:group-focus-within:opacity-100">
                    <IconActionButton
                      label={t("dictionary.modal.editWord", { word: entry.word })}
                      disabled={busy}
                      onClick={() => onEdit(entry)}
                    >
                      <Pencil size={15} strokeWidth={1.75} />
                    </IconActionButton>
                    <IconActionButton
                      label={t("dictionary.modal.deleteWord", { word: entry.word })}
                      disabled={busy}
                      tone="danger"
                      onClick={() => onDelete(entry.id)}
                    >
                      <Trash2 size={15} strokeWidth={1.75} />
                    </IconActionButton>
                  </div>
                </td>
              </>
            )}
          />
        </table>
      </div>
    </div>
  );
}
