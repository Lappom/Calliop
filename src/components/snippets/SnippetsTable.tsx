import type { ReactNode } from "react";
import { ArrowRight, Pencil, Trash2 } from "lucide-react";
import { useUiLocale } from "../../i18n/useUiLocale";
import type { Snippet } from "../../hooks/useSnippets";
import { AnimatedTableBody } from "../motion/AnimatedTableBody";
import { BadgePill } from "../ui/BadgePill";
import { containsSnippetVariables } from "./snippetVariables";

interface SnippetsTableProps {
  snippets: Snippet[];
  busy: boolean;
  onEdit: (snippet: Snippet) => void;
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

export function SnippetsTable({
  snippets,
  busy,
  onEdit,
  onDelete,
}: SnippetsTableProps) {
  const { t } = useUiLocale();

  return (
    <div className="overflow-hidden rounded-lg border border-hairline-strong bg-surface-card">
      <div className="overflow-x-auto">
        <table className="w-full min-w-[520px] border-collapse">
          <AnimatedTableBody
            items={snippets}
            getRowKey={(entry) => entry.id}
            renderRow={(entry) => (
              <>
                <td className="min-w-0 px-4 py-3.5">
                  <div className="flex min-w-0 items-center gap-2 sm:gap-3">
                    <span className="shrink-0 text-body-sm font-medium text-ink">
                      {entry.trigger}
                    </span>
                    <ArrowRight
                      size={14}
                      className="shrink-0 text-ash"
                      aria-hidden
                    />
                    <span className="flex min-w-0 items-center gap-2">
                      <span className="min-w-0 truncate text-body-sm text-charcoal">
                        {entry.content}
                      </span>
                      {containsSnippetVariables(entry.content) && (
                        <BadgePill className="shrink-0">
                          {t("snippets.badge.variables")}
                        </BadgePill>
                      )}
                    </span>
                  </div>
                </td>
                <td className="w-20 shrink-0 px-2 py-2 sm:w-24">
                  <div className="flex items-center justify-end gap-0.5 opacity-100 transition-opacity sm:opacity-0 sm:group-hover:opacity-100 sm:group-focus-within:opacity-100">
                    <IconActionButton
                      label={t("snippets.modal.editTrigger", {
                        trigger: entry.trigger,
                      })}
                      disabled={busy}
                      onClick={() => onEdit(entry)}
                    >
                      <Pencil size={15} strokeWidth={1.75} />
                    </IconActionButton>
                    <IconActionButton
                      label={t("snippets.modal.deleteTrigger", {
                        trigger: entry.trigger,
                      })}
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
