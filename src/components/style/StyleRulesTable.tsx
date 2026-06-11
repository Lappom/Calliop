import { ArrowRight, Trash2 } from "lucide-react";
import type { AppContextRule } from "../../hooks/useAppContext";
import { MATCH_TYPE_LABELS } from "./styleUtils";
import { ToneBadge } from "./ToneBadge";

interface StyleRulesTableProps {
  rules: AppContextRule[];
  busy: boolean;
  onDelete: (id: number) => void;
}

export function StyleRulesTable({
  rules,
  busy,
  onDelete,
}: StyleRulesTableProps) {
  return (
    <div className="overflow-hidden rounded-lg border border-hairline-strong bg-surface-card">
      <div className="overflow-x-auto">
        <table className="w-full min-w-[520px] border-collapse">
          <tbody>
            {rules.map((rule) => (
              <tr
                key={rule.id}
                className="group border-b border-divider-soft transition-colors last:border-b-0 hover:bg-surface-elevated/50"
              >
                <td className="min-w-0 px-4 py-3.5">
                  <div className="flex min-w-0 items-center gap-2 sm:gap-3">
                    <span className="shrink-0 text-body-sm font-medium text-ink">
                      {rule.pattern}
                    </span>
                    <ArrowRight
                      size={14}
                      className="shrink-0 text-ash"
                      aria-hidden
                    />
                    <span className="shrink-0 text-caption text-ash">
                      {MATCH_TYPE_LABELS[rule.matchType]}
                    </span>
                    <ArrowRight
                      size={14}
                      className="hidden shrink-0 text-ash sm:inline"
                      aria-hidden
                    />
                    <ToneBadge tone={rule.tone} className="hidden sm:inline-flex" />
                  </div>
                  <ToneBadge tone={rule.tone} className="mt-2 sm:hidden" />
                </td>
                <td className="w-12 shrink-0 px-2 py-2">
                  <div className="flex justify-end opacity-100 transition-opacity sm:opacity-0 sm:group-hover:opacity-100 sm:group-focus-within:opacity-100">
                    <button
                      type="button"
                      aria-label={`Supprimer la règle ${rule.pattern}`}
                      disabled={busy}
                      onClick={() => onDelete(rule.id)}
                      className={[
                        "inline-flex size-8 items-center justify-center rounded-md border border-transparent",
                        "text-charcoal transition-colors duration-150",
                        "hover:border-hairline-strong hover:bg-surface-elevated hover:text-accent-red",
                        "disabled:cursor-not-allowed disabled:opacity-40",
                      ].join(" ")}
                    >
                      <Trash2 size={15} strokeWidth={1.75} />
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
