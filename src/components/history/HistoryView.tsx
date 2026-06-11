import { ArrowDownUp, RefreshCw, Search } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useHistory } from "../../hooks/useHistory";
import { SnippetListToolbarButton } from "../snippets/SnippetListToolbarButton";
import { SectionGlow } from "../layout/SectionGlow";
import { Kbd } from "../ui/Kbd";
import { TextInput } from "../ui/TextInput";
import { HistoryList } from "./HistoryList";
import { HistoryPagination } from "./HistoryPagination";
import { HistoryStatsBar } from "./HistoryStatsBar";
import {
  groupHistoryEntries,
  HISTORY_SORT_LABELS,
  nextHistorySort,
  sortHistoryEntries,
  type HistorySort,
} from "./historyUtils";

export function HistoryView() {
  const [searchQuery, setSearchQuery] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [sort, setSort] = useState<HistorySort>("recent");
  const {
    entries,
    loaded,
    busy,
    page,
    totalCount,
    pageSize,
    errorMessage,
    entryFeedback,
    loadEntries,
    goToPage,
    copyEntry,
    reinjectEntry,
  } = useHistory();

  useEffect(() => {
    const handle = window.setTimeout(() => {
      void loadEntries({ query: searchQuery });
    }, 300);
    return () => window.clearTimeout(handle);
  }, [searchQuery, loadEntries]);

  const visibleGroups = useMemo(() => {
    const sorted = sortHistoryEntries(entries, sort);
    return groupHistoryEntries(sorted);
  }, [entries, sort]);

  const hasEntries = loaded && totalCount > 0;
  const hasPageEntries = loaded && entries.length > 0;
  const isSearching = searchQuery.trim().length > 0;

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">Historique</h1>
        <p className="text-body-sm text-charcoal">
          Retrouvez vos dictées passées. Utilisez les actions sur chaque entrée
          pour copier ou réinjecter le texte dans l&apos;application active.
        </p>
      </header>

      {hasEntries && (
        <HistoryStatsBar entries={entries} totalCount={totalCount} />
      )}

      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-caption m-0 text-ash">
          <Kbd>Alt</Kbd> + <Kbd>Espace</Kbd> pour dicter · survolez une entrée
          pour copier ou réinjecter
        </p>

        {loaded && (
          <div className="flex items-center gap-1">
            <SnippetListToolbarButton
              label="Rechercher"
              active={searchOpen}
              disabled={busy}
              onClick={() => {
                setSearchOpen((current) => {
                  if (current) {
                    setSearchQuery("");
                    void loadEntries({ query: "" });
                  }
                  return !current;
                });
              }}
            >
              <Search size={16} strokeWidth={1.75} />
            </SnippetListToolbarButton>
            {hasEntries && (
              <>
                <SnippetListToolbarButton
                  label={HISTORY_SORT_LABELS[sort]}
                  disabled={busy}
                  onClick={() => setSort((current) => nextHistorySort(current))}
                >
                  <ArrowDownUp size={16} strokeWidth={1.75} />
                </SnippetListToolbarButton>
                <SnippetListToolbarButton
                  label="Actualiser l'historique"
                  disabled={busy}
                  onClick={() => {
                    void loadEntries({ query: searchQuery, page });
                  }}
                >
                  <RefreshCw
                    size={16}
                    strokeWidth={1.75}
                    className={busy ? "animate-spin" : undefined}
                  />
                </SnippetListToolbarButton>
              </>
            )}
          </div>
        )}
      </div>

      {searchOpen && (
        <TextInput
          label="Rechercher dans l'historique"
          value={searchQuery}
          onChange={(event) => setSearchQuery(event.target.value)}
          placeholder="Mots-clés, application, extrait de texte…"
          disabled={!loaded || busy}
          autoFocus
        />
      )}

      {errorMessage && (
        <p className="text-body-sm text-accent-red">{errorMessage}</p>
      )}

      {!loaded && (
        <p className="text-body-sm text-charcoal">Chargement…</p>
      )}

      {loaded && totalCount === 0 && (
        <SectionGlow glow="green">
          <div className="rounded-lg border border-hairline-strong bg-surface-card p-6 sm:p-8">
            <p className="text-body-md m-0 text-charcoal">
              {isSearching
                ? `Aucun résultat pour « ${searchQuery.trim()} ».`
                : "Aucune dictée enregistrée pour l'instant."}
            </p>
            {!isSearching && (
              <p className="text-body-sm mt-3 text-ash">
                Placez le curseur dans une application, puis appuyez sur{" "}
                <Kbd>Alt</Kbd> + <Kbd>Espace</Kbd> pour commencer.
              </p>
            )}
          </div>
        </SectionGlow>
      )}

      {hasPageEntries && (
        <>
          <HistoryList
            groups={visibleGroups}
            busy={busy}
            entryFeedback={entryFeedback}
            onCopy={(id) => {
              void copyEntry(id);
            }}
            onReinject={(id) => {
              void reinjectEntry(id);
            }}
          />
          <HistoryPagination
            page={page}
            pageSize={pageSize}
            total={totalCount}
            disabled={busy}
            onPageChange={goToPage}
          />
        </>
      )}
    </div>
  );
}
