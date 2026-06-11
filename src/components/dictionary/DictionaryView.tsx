import {
  ArrowDownUp,
  Filter,
  Plus,
} from "lucide-react";
import { useMemo, useState } from "react";
import { useUiLocale } from "../../i18n/useUiLocale";
import type { DictionaryWord } from "../../hooks/useDictionary";
import { useDictionary } from "../../hooks/useDictionary";
import { useRefreshSpin } from "../../hooks/useRefreshSpin";
import { EmptyStateCard } from "../motion/EmptyStateCard";
import { NoResultsCard } from "../motion/NoResultsCard";
import { Stagger } from "../motion/Stagger";
import { SnippetListToolbarButton } from "../snippets/SnippetListToolbarButton";
import { Button } from "../ui/Button";
import { ExpandableSearchField } from "../ui/ExpandableSearchField";
import { RefreshIcon } from "../ui/RefreshIcon";
import { toolbarMenuOptions } from "../ui/toolbarMenu";
import { DictionaryTable } from "./DictionaryTable";
import { DictionaryWordModal } from "./DictionaryWordModal";
import {
  DICTIONARY_SORT_ORDER,
  filterDictionaryWords,
  getDictionaryFilterLabels,
  getDictionarySortLabels,
  nextDictionarySort,
  sortDictionaryWords,
  SOURCE_FILTER_ORDER,
  type DictionarySort,
  type DictionarySourceFilter,
} from "./dictionaryUtils";

type ModalState =
  | { mode: "closed" }
  | { mode: "create" }
  | { mode: "edit"; entry: DictionaryWord };

export function DictionaryView() {
  const { t, intlLocale } = useUiLocale();
  const [modalState, setModalState] = useState<ModalState>({ mode: "closed" });
  const [modalError, setModalError] = useState<string | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [sort, setSort] = useState<DictionarySort>("alpha-asc");
  const [sourceFilter, setSourceFilter] = useState<DictionarySourceFilter>("all");
  const {
    words,
    loaded,
    busy,
    errorMessage,
    addWord,
    updateWord,
    removeWord,
    reload,
  } = useDictionary();

  const { spinning: refreshSpinning, runRefresh } = useRefreshSpin(busy);

  const sortLabels = useMemo(() => getDictionarySortLabels(t), [t]);

  const sourceFilterLabels = useMemo(
    () => getDictionaryFilterLabels(t),
    [t],
  );

  const visibleWords = useMemo(() => {
    const filtered = filterDictionaryWords(
      words,
      searchQuery,
      sourceFilter,
      intlLocale,
    );
    return sortDictionaryWords(filtered, sort, intlLocale);
  }, [words, searchQuery, sourceFilter, sort, intlLocale]);

  const closeModal = () => {
    setModalState({ mode: "closed" });
    setModalError(null);
  };

  const handleSubmit = async (
    word: string,
    misspelling?: string,
  ): Promise<boolean> => {
    setModalError(null);
    if (modalState.mode === "create") {
      const inserted = await addWord(word, misspelling);
      if (!inserted) {
        setModalError(t("dictionary.errors.alreadyExists"));
      }
      return inserted;
    }
    if (modalState.mode === "edit") {
      const updated = await updateWord(modalState.entry.id, word);
      if (!updated) {
        setModalError(t("dictionary.errors.duplicateWord"));
      }
      return updated;
    }
    return false;
  };

  const cycleSourceFilter = () => {
    setSourceFilter((current) => {
      if (current === "all") return "manual";
      if (current === "manual") return "learned";
      return "all";
    });
  };

  return (
    <>
    <Stagger className="flex flex-col gap-8" itemMotion="fade">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">{t("dictionary.title")}</h1>
        <p className="text-body-sm text-charcoal">{t("dictionary.subtitle")}</p>
      </header>

      <div className="flex flex-wrap items-center justify-between gap-3">
        <Button
          type="button"
          variant="primary"
          className="inline-flex items-center gap-1.5"
          disabled={!loaded || busy}
          onClick={() => {
            setModalError(null);
            setModalState({ mode: "create" });
          }}
        >
          <Plus size={16} aria-hidden />
          {t("dictionary.newWord")}
        </Button>

        {loaded && words.length > 0 && (
          <div className="flex items-center gap-1">
            <ExpandableSearchField
              open={searchOpen}
              disabled={busy}
              label={t("dictionary.searchLabel")}
              placeholder={t("dictionary.searchPlaceholder")}
              value={searchQuery}
              onChange={setSearchQuery}
              onOpenChange={(next) => {
                if (!next) {
                  setSearchQuery("");
                }
                setSearchOpen(next);
              }}
            />
            <SnippetListToolbarButton
              label={sourceFilterLabels[sourceFilter]}
              active={sourceFilter !== "all"}
              disabled={busy}
              onClick={cycleSourceFilter}
              onMenuSelect={setSourceFilter}
              menuTitle={t("common.filters")}
              menuOptions={toolbarMenuOptions(
                sourceFilterLabels,
                SOURCE_FILTER_ORDER,
              )}
              activeMenuValue={sourceFilter}
            >
              <Filter size={16} strokeWidth={1.75} />
            </SnippetListToolbarButton>
            <SnippetListToolbarButton
              label={sortLabels[sort]}
              disabled={busy}
              onClick={() => setSort((current) => nextDictionarySort(current))}
              onMenuSelect={setSort}
              menuTitle={t("common.sort")}
              menuOptions={toolbarMenuOptions(
                sortLabels,
                DICTIONARY_SORT_ORDER,
              )}
              activeMenuValue={sort}
            >
              <ArrowDownUp size={16} strokeWidth={1.75} />
            </SnippetListToolbarButton>
            <SnippetListToolbarButton
              label={t("common.refreshList")}
              disabled={busy || refreshSpinning}
              onClick={() => {
                void runRefresh(() => reload());
              }}
            >
              <RefreshIcon spinning={refreshSpinning} />
            </SnippetListToolbarButton>
          </div>
        )}
      </div>

      {errorMessage && (
        <p className="text-body-sm text-accent-red">{errorMessage}</p>
      )}

      {!loaded && (
        <p className="text-body-sm text-charcoal">{t("common.loading")}</p>
      )}

      {loaded && words.length === 0 && (
        <EmptyStateCard glow="blue">
          <p className="text-body-md m-0 text-charcoal">{t("dictionary.empty")}</p>
          <Button
            type="button"
            variant="primary"
            className="mt-4"
            disabled={busy}
            onClick={() => {
              setModalError(null);
              setModalState({ mode: "create" });
            }}
          >
            {t("dictionary.addWord")}
          </Button>
        </EmptyStateCard>
      )}

      {loaded && words.length > 0 && (
        <div className="flex flex-col gap-3">
          {visibleWords.length === 0 ? (
            <NoResultsCard>
              <p className="text-body-sm m-0 text-charcoal">
                {t("common.noResultsFilters")}
              </p>
            </NoResultsCard>
          ) : (
            <DictionaryTable
              words={visibleWords}
              busy={busy}
              onEdit={(entry) => {
                setModalError(null);
                setModalState({ mode: "edit", entry });
              }}
              onDelete={(id) => {
                void removeWord(id);
              }}
            />
          )}
        </div>
      )}
    </Stagger>

      <DictionaryWordModal
        open={modalState.mode !== "closed"}
        onClose={closeModal}
        busy={busy}
        errorMessage={modalError}
        mode={modalState.mode === "edit" ? "edit" : "create"}
        initialWord={
          modalState.mode === "edit" ? modalState.entry.word : ""
        }
        initialMisspelling={
          modalState.mode === "edit"
            ? modalState.entry.misspelling ?? null
            : null
        }
        onSubmit={handleSubmit}
      />
    </>
  );
}
