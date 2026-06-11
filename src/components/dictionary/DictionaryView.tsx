import {
  ArrowDownUp,
  BookOpen,
  Plus,
  RefreshCw,
  Search,
} from "lucide-react";
import { useMemo, useState } from "react";
import type { DictionaryWord } from "../../hooks/useDictionary";
import { useDictionary } from "../../hooks/useDictionary";
import { SnippetListToolbarButton } from "../snippets/SnippetListToolbarButton";
import { SectionGlow } from "../layout/SectionGlow";
import { Button } from "../ui/Button";
import { TextInput } from "../ui/TextInput";
import { DictionaryTable } from "./DictionaryTable";
import { DictionaryWordModal } from "./DictionaryWordModal";
import {
  DICTIONARY_SORT_LABELS,
  filterDictionaryWords,
  nextDictionarySort,
  sortDictionaryWords,
  SOURCE_FILTER_LABELS,
  type DictionarySort,
  type DictionarySourceFilter,
} from "./dictionaryUtils";

type ModalState =
  | { mode: "closed" }
  | { mode: "create" }
  | { mode: "edit"; entry: DictionaryWord };

export function DictionaryView() {
  const [modalState, setModalState] = useState<ModalState>({ mode: "closed" });
  const [modalError, setModalError] = useState<string | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [sort, setSort] = useState<DictionarySort>("alpha-asc");
  const [sourceFilter, setSourceFilter] =
    useState<DictionarySourceFilter>("all");
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

  const visibleWords = useMemo(() => {
    const filtered = filterDictionaryWords(words, searchQuery, sourceFilter);
    return sortDictionaryWords(filtered, sort);
  }, [words, searchQuery, sourceFilter, sort]);

  const hasWords = loaded && words.length > 0;
  const hasVisibleWords = loaded && visibleWords.length > 0;
  const isFiltering =
    searchQuery.trim().length > 0 || sourceFilter !== "all";

  const openCreateModal = () => {
    setModalError(null);
    setModalState({ mode: "create" });
  };

  const closeModal = () => {
    setModalState({ mode: "closed" });
    setModalError(null);
  };

  const handleSubmit = async (word: string) => {
    setModalError(null);
    if (modalState.mode === "create") {
      const inserted = await addWord(word);
      if (!inserted) {
        setModalError("Ce mot est déjà dans le dictionnaire.");
      }
      return inserted;
    }
    if (modalState.mode === "edit") {
      const updated = await updateWord(modalState.entry.id, word);
      if (!updated) {
        setModalError("Ce mot est déjà dans le dictionnaire.");
      }
      return updated;
    }
    return false;
  };

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">Dictionnaire</h1>
        <p className="text-body-sm text-charcoal">
          Mots et noms propres injectés dans le prompt Whisper pour améliorer
          la transcription — ajoutez-les ici ou via une correction sur
          l&apos;accueil.
        </p>
      </header>

      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-3">
          <Button
            type="button"
            variant="primary"
            className="inline-flex items-center gap-1.5"
            disabled={!loaded || busy}
            onClick={openCreateModal}
          >
            <Plus size={16} aria-hidden />
            Nouveau mot
          </Button>
        </div>

        {loaded && hasWords && (
          <div className="flex items-center gap-1">
            <SnippetListToolbarButton
              label="Rechercher"
              active={searchOpen}
              disabled={busy}
              onClick={() => {
                setSearchOpen((current) => {
                  if (current) {
                    setSearchQuery("");
                  }
                  return !current;
                });
              }}
            >
              <Search size={16} strokeWidth={1.75} />
            </SnippetListToolbarButton>
            <SnippetListToolbarButton
              label={DICTIONARY_SORT_LABELS[sort]}
              disabled={busy}
              onClick={() => setSort((current) => nextDictionarySort(current))}
            >
              <ArrowDownUp size={16} strokeWidth={1.75} />
            </SnippetListToolbarButton>
            <SnippetListToolbarButton
              label="Actualiser le dictionnaire"
              disabled={busy}
              onClick={() => {
                void reload();
              }}
            >
              <RefreshCw
                size={16}
                strokeWidth={1.75}
                className={busy ? "animate-spin" : undefined}
              />
            </SnippetListToolbarButton>
          </div>
        )}
      </div>

      {hasWords && (
        <div
          className="flex flex-wrap gap-2"
          role="group"
          aria-label="Filtrer par source"
        >
          {(Object.keys(SOURCE_FILTER_LABELS) as DictionarySourceFilter[]).map(
            (filter) => (
              <button
                key={filter}
                type="button"
                disabled={busy}
                aria-pressed={sourceFilter === filter}
                onClick={() => setSourceFilter(filter)}
                className={[
                  "rounded-full border px-3 py-1.5",
                  "font-[family-name:var(--font-ui)] text-xs font-medium leading-normal",
                  "transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-40",
                  sourceFilter === filter
                    ? "border-hairline-strong bg-surface-elevated text-ink"
                    : "border-transparent bg-surface-card text-charcoal hover:border-hairline-strong hover:text-ink",
                ].join(" ")}
              >
                {SOURCE_FILTER_LABELS[filter]}
              </button>
            ),
          )}
        </div>
      )}

      {loaded && hasWords && searchOpen && (
        <TextInput
          label="Rechercher un mot"
          value={searchQuery}
          onChange={(event) => setSearchQuery(event.target.value)}
          placeholder="Filtrer par mot…"
          disabled={busy}
          autoFocus
        />
      )}

      {errorMessage && (
        <p className="text-body-sm text-accent-red">{errorMessage}</p>
      )}

      {!loaded && (
        <p className="text-body-sm text-charcoal">Chargement…</p>
      )}

      {loaded && !hasWords && (
        <SectionGlow glow="orange">
          <div className="rounded-lg border border-hairline-strong bg-surface-card p-6 sm:p-8">
            <div className="flex items-start gap-4">
              <span className="inline-flex size-10 shrink-0 items-center justify-center rounded-lg border border-hairline-strong bg-surface-elevated text-charcoal">
                <BookOpen size={18} strokeWidth={1.75} aria-hidden />
              </span>
              <div>
                <p className="text-body-md m-0 text-ink">
                  Votre dictionnaire est vide
                </p>
                <p className="text-body-sm mt-2 text-charcoal">
                  Ajoutez des noms propres, acronymes ou termes techniques que
                  Whisper peine à reconnaître. Vous pouvez aussi corriger une
                  dictée sur l&apos;accueil pour enregistrer automatiquement la
                  bonne orthographe.
                </p>
                <Button
                  type="button"
                  variant="primary"
                  className="mt-4"
                  disabled={busy}
                  onClick={openCreateModal}
                >
                  Ajouter un mot
                </Button>
              </div>
            </div>
          </div>
        </SectionGlow>
      )}

      {loaded && hasWords && !hasVisibleWords && (
        <div className="rounded-lg border border-hairline-strong bg-surface-card px-4 py-8 text-center">
          <p className="text-body-sm m-0 text-charcoal">
            {isFiltering
              ? searchQuery.trim()
                ? `Aucun résultat pour « ${searchQuery.trim()} ».`
                : "Aucun mot ne correspond à vos filtres."
              : "Aucun mot à afficher."}
          </p>
        </div>
      )}

      {hasVisibleWords && (
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

      <DictionaryWordModal
        open={modalState.mode !== "closed"}
        onClose={closeModal}
        busy={busy}
        errorMessage={modalError}
        mode={modalState.mode === "edit" ? "edit" : "create"}
        initialWord={
          modalState.mode === "edit" ? modalState.entry.word : ""
        }
        onSubmit={handleSubmit}
      />
    </div>
  );
}
