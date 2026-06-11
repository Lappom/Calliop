import {
  ArrowDownUp,
  Plus,
  RefreshCw,
  Search,
} from "lucide-react";
import { useMemo, useState } from "react";
import type { Snippet } from "../../hooks/useSnippets";
import { useSnippets } from "../../hooks/useSnippets";
import { SectionGlow } from "../layout/SectionGlow";
import { Button } from "../ui/Button";
import { TextInput } from "../ui/TextInput";
import { SnippetModal } from "./SnippetModal";
import { SnippetListToolbarButton } from "./SnippetListToolbarButton";
import {
  filterSnippets,
  sortSnippets,
  type SnippetSort,
} from "./snippetListUtils";
import { SnippetsTable } from "./SnippetsTable";

type ModalState =
  | { mode: "closed" }
  | { mode: "create" }
  | { mode: "edit"; snippet: Snippet };

const SORT_LABELS: Record<SnippetSort, string> = {
  "trigger-asc": "Tri A → Z",
  "trigger-desc": "Tri Z → A",
  recent: "Plus récents",
};

function nextSort(current: SnippetSort): SnippetSort {
  if (current === "trigger-asc") return "trigger-desc";
  if (current === "trigger-desc") return "recent";
  return "trigger-asc";
}

export function SnippetsView() {
  const [modalState, setModalState] = useState<ModalState>({ mode: "closed" });
  const [modalError, setModalError] = useState<string | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [sort, setSort] = useState<SnippetSort>("trigger-asc");
  const {
    snippets,
    loaded,
    busy,
    errorMessage,
    fileInputRef,
    addSnippet,
    updateSnippet,
    removeSnippet,
    exportSnippets,
    openImportDialog,
    handleImportFile,
    reload,
  } = useSnippets();

  const visibleSnippets = useMemo(() => {
    const filtered = filterSnippets(snippets, searchQuery);
    return sortSnippets(filtered, sort);
  }, [snippets, searchQuery, sort]);

  const closeModal = () => {
    setModalState({ mode: "closed" });
    setModalError(null);
  };

  const handleSubmit = async (trigger: string, content: string) => {
    setModalError(null);
    if (modalState.mode === "create") {
      const inserted = await addSnippet(trigger, content);
      if (!inserted) {
        setModalError("Ce déclencheur existe déjà.");
      }
      return inserted;
    }
    if (modalState.mode === "edit") {
      const updated = await updateSnippet(
        modalState.snippet.id,
        trigger,
        content,
      );
      if (!updated) {
        setModalError("Ce déclencheur existe déjà.");
      }
      return updated;
    }
    return false;
  };

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">Snippets</h1>
        <p className="text-body-sm text-charcoal">
          Définissez des déclencheurs vocaux qui insèrent un texte complet après
          la transcription.
        </p>
      </header>

      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-3">
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
            Nouveau snippet
          </Button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".json,application/json"
            className="hidden"
            onChange={(event) => {
              void handleImportFile(event);
            }}
          />
          <Button
            type="button"
            variant="ghost"
            disabled={!loaded || busy}
            onClick={openImportDialog}
          >
            Importer JSON
          </Button>
          <Button
            type="button"
            variant="ghost"
            disabled={!loaded || busy || snippets.length === 0}
            onClick={() => {
              void exportSnippets();
            }}
          >
            Exporter JSON
          </Button>
        </div>

        {loaded && snippets.length > 0 && (
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
              label={SORT_LABELS[sort]}
              disabled={busy}
              onClick={() => setSort((current) => nextSort(current))}
            >
              <ArrowDownUp size={16} strokeWidth={1.75} />
            </SnippetListToolbarButton>
            <SnippetListToolbarButton
              label="Actualiser la liste"
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

      {loaded && snippets.length > 0 && searchOpen && (
        <TextInput
          label="Rechercher un snippet"
          value={searchQuery}
          onChange={(event) => setSearchQuery(event.target.value)}
          placeholder="Déclencheur ou texte…"
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

      {loaded && snippets.length === 0 && (
        <SectionGlow glow="blue">
          <div className="rounded-lg border border-hairline-strong bg-surface-card p-6 sm:p-8">
            <p className="text-body-md m-0 text-charcoal">
              Aucun snippet enregistré. Créez un déclencheur vocal ou importez
              un fichier JSON.
            </p>
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
              Créer un snippet
            </Button>
          </div>
        </SectionGlow>
      )}

      {loaded && snippets.length > 0 && (
        <div className="flex flex-col gap-3">
          {visibleSnippets.length === 0 ? (
            <div className="rounded-lg border border-hairline-strong bg-surface-card px-4 py-8 text-center">
              <p className="text-body-sm m-0 text-charcoal">
                Aucun résultat pour « {searchQuery.trim()} ».
              </p>
            </div>
          ) : (
            <SnippetsTable
              snippets={visibleSnippets}
              busy={busy}
              onEdit={(snippet) => {
                setModalError(null);
                setModalState({ mode: "edit", snippet });
              }}
              onDelete={(id) => {
                void removeSnippet(id);
              }}
            />
          )}
        </div>
      )}

      <SnippetModal
        open={modalState.mode !== "closed"}
        onClose={closeModal}
        busy={busy}
        errorMessage={modalError}
        mode={modalState.mode === "edit" ? "edit" : "create"}
        initialTrigger={
          modalState.mode === "edit" ? modalState.snippet.trigger : ""
        }
        initialContent={
          modalState.mode === "edit" ? modalState.snippet.content : ""
        }
        onSubmit={handleSubmit}
      />
    </div>
  );
}
