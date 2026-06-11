import { ArrowDownUp, RefreshCw, Search } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useUiLocale } from "../../i18n/useUiLocale";
import { useHistory } from "../../hooks/useHistory";
import { SnippetListToolbarButton } from "../snippets/SnippetListToolbarButton";
import { SectionGlow } from "../layout/SectionGlow";
import { TextInput } from "../ui/TextInput";
import { toolbarMenuOptions } from "../ui/toolbarMenu";
import { HistoryList } from "./HistoryList";
import { HistoryPagination } from "./HistoryPagination";
import { HistoryStatsBar } from "./HistoryStatsBar";
import {
  getHistoryGroupLabels,
  groupHistoryEntries,
  HISTORY_SORT_ORDER,
  nextHistorySort,
  sortHistoryEntries,
  type HistorySort,
} from "./historyUtils";

export function HistoryView() {
  const { t, intlLocale } = useUiLocale();
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

  const sortLabels = useMemo(
    (): Record<HistorySort, string> => ({
      recent: t("history.sort.recent"),
      oldest: t("history.sort.oldest"),
      longest: t("history.sort.longest"),
    }),
    [t],
  );

  useEffect(() => {
    const handle = window.setTimeout(() => {
      void loadEntries({ query: searchQuery });
    }, 300);
    return () => window.clearTimeout(handle);
  }, [searchQuery, loadEntries]);

  const groupLabels = useMemo(() => getHistoryGroupLabels(t), [t]);

  const visibleGroups = useMemo(() => {
    const sorted = sortHistoryEntries(entries, sort);
    return groupHistoryEntries(sorted, intlLocale, groupLabels);
  }, [entries, sort, intlLocale, groupLabels]);

  const hasEntries = loaded && totalCount > 0;
  const hasPageEntries = loaded && entries.length > 0;
  const isSearching = searchQuery.trim().length > 0;

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">{t("history.title")}</h1>
        <p className="text-body-sm text-charcoal">{t("history.subtitle")}</p>
      </header>

      {hasEntries && (
        <HistoryStatsBar entries={entries} totalCount={totalCount} />
      )}

      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-caption m-0 text-ash">
          {t("keys.dictateHint", {
            alt: t("keys.modifiers.alt"),
            space: t("keys.space"),
          })}
        </p>

        {loaded && (
          <div className="flex items-center gap-1">
            <SnippetListToolbarButton
              label={t("common.search")}
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
                  label={sortLabels[sort]}
                  disabled={busy}
                  onClick={() => setSort((current) => nextHistorySort(current))}
                  onMenuSelect={setSort}
                  menuTitle={t("common.sort")}
                  menuOptions={toolbarMenuOptions(
                    sortLabels,
                    HISTORY_SORT_ORDER,
                  )}
                  activeMenuValue={sort}
                >
                  <ArrowDownUp size={16} strokeWidth={1.75} />
                </SnippetListToolbarButton>
                <SnippetListToolbarButton
                  label={t("history.refresh")}
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
          label={t("history.searchLabel")}
          value={searchQuery}
          onChange={(event) => setSearchQuery(event.target.value)}
          placeholder={t("history.searchPlaceholder")}
          disabled={!loaded || busy}
          autoFocus
        />
      )}

      {errorMessage && (
        <p className="text-body-sm text-accent-red">{errorMessage}</p>
      )}

      {!loaded && (
        <p className="text-body-sm text-charcoal">{t("common.loading")}</p>
      )}

      {loaded && totalCount === 0 && (
        <SectionGlow glow="green">
          <div className="rounded-lg border border-hairline-strong bg-surface-card p-6 sm:p-8">
            <p className="text-body-md m-0 text-charcoal">
              {isSearching
                ? t("common.noResults", { query: searchQuery.trim() })
                : t("history.empty")}
            </p>
            {!isSearching && (
              <p className="text-body-sm mt-3 text-ash">
                {t("keys.dictateStartHint", {
                  alt: t("keys.modifiers.alt"),
                  space: t("keys.space"),
                })}
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
