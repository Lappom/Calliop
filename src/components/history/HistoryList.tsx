import { HistoryEntryRow } from "./HistoryEntryRow";
import type { HistoryGroup } from "./historyUtils";

interface HistoryListProps {
  groups: HistoryGroup[];
  busy: boolean;
  entryFeedback: Record<number, "copied" | "injected">;
  onCopy: (id: number) => void;
  onReinject: (id: number) => void;
}

export function HistoryList({
  groups,
  busy,
  entryFeedback,
  onCopy,
  onReinject,
}: HistoryListProps) {
  return (
    <div className="flex flex-col gap-8">
      {groups.map((group) => (
        <section key={group.label} aria-labelledby={`history-group-${group.label}`}>
          <h2
            id={`history-group-${group.label}`}
            className="text-caption mb-3 font-medium uppercase tracking-wider text-ash"
          >
            {group.label}
          </h2>
          <ul className="m-0 flex list-none flex-col gap-3 p-0">
            {group.entries.map((entry) => (
              <li key={entry.id}>
                <HistoryEntryRow
                  entry={entry}
                  busy={busy}
                  feedback={entryFeedback[entry.id]}
                  onCopy={onCopy}
                  onReinject={onReinject}
                />
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
