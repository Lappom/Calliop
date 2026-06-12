import { ArrowDownUp } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useUiLocale } from "../../i18n/useUiLocale";
import { useHistory } from "../../hooks/useHistory";
import { useRefreshSpin } from "../../hooks/useRefreshSpin";
import { EmptyStateCard } from "../motion/EmptyStateCard";
import { Stagger } from "../motion/Stagger";
import { SnippetListToolbarButton } from "../snippets/SnippetListToolbarButton";
import { ExpandableSearchField } from "../ui/ExpandableSearchField";
import { RefreshIcon } from "../ui/RefreshIcon";
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
    actionEntryId,
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

  const { spinning: refreshSpinning, runRefresh } = useRefreshSpin();

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
    <Stagger className="flex flex-col gap-8" itemMotion="fade">
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
            <ExpandableSearchField
              open={searchOpen}
              disabled={!loaded}
              label={t("history.searchLabel")}
              placeholder={t("history.searchPlaceholder")}
              value={searchQuery}
              onChange={setSearchQuery}
              onOpenChange={(next) => {
                if (!next) {
                  setSearchQuery("");
                  void loadEntries({ query: "" });
                }
                setSearchOpen(next);
              }}
            />
            {hasEntries && (
              <>
                <SnippetListToolbarButton
                  label={sortLabels[sort]}
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
                  disabled={refreshSpinning}
                  onClick={() => {
                    void runRefresh(() =>
                      loadEntries({ query: searchQuery, page }),
                    );
                  }}
                >
                  <RefreshIcon spinning={refreshSpinning} />
                </SnippetListToolbarButton>
              </>
            )}
          </div>
        )}
      </div>

      {errorMessage && (
        <p className="text-body-sm text-accent-red">{errorMessage}</p>
      )}

      {!loaded && (
        <p className="text-body-sm text-charcoal">{t("common.loading")}</p>
      )}

      {loaded && totalCount === 0 && (
        <EmptyStateCard glow="green">
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
        </EmptyStateCard>
      )}

      {hasPageEntries && (
        <>
          <HistoryList
            groups={visibleGroups}
            actionEntryId={actionEntryId}
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
            onPageChange={goToPage}
          />
        </>
      )}
    </Stagger>
  );
}
