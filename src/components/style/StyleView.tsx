import {
  ArrowDownUp,
  Plus,
  RefreshCw,
  Search,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type {
  AppContextMatchType,
  ToneProfile,
} from "../../hooks/useAppContext";
import { useAppContext } from "../../hooks/useAppContext";
import { SnippetListToolbarButton } from "../snippets/SnippetListToolbarButton";
import { SectionGlow } from "../layout/SectionGlow";
import { Button } from "../ui/Button";
import { TextInput } from "../ui/TextInput";
import { toolbarMenuOptions } from "../ui/toolbarMenu";
import { ActiveWindowCard } from "./ActiveWindowCard";
import { StyleRuleModal } from "./StyleRuleModal";
import { StyleRulesTable } from "./StyleRulesTable";
import {
  filterStyleRules,
  nextStyleSort,
  sortStyleRules,
  STYLE_SORT_LABELS,
  STYLE_SORT_ORDER,
  type StyleRuleSort,
} from "./styleUtils";

interface ModalDefaults {
  pattern: string;
  matchType: AppContextMatchType;
  tone: ToneProfile;
}

const DEFAULT_MODAL: ModalDefaults = {
  pattern: "",
  matchType: "exe",
  tone: "casual",
};

export function StyleView() {
  const [modalOpen, setModalOpen] = useState(false);
  const [modalError, setModalError] = useState<string | null>(null);
  const [modalDefaults, setModalDefaults] = useState<ModalDefaults>(DEFAULT_MODAL);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [sort, setSort] = useState<StyleRuleSort>("recent");
  const {
    rules,
    activeWindow,
    loaded,
    busy,
    errorMessage,
    addRule,
    removeRule,
    refreshActiveWindow,
    reload,
  } = useAppContext();

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      void refreshActiveWindow();
    }, 1000);
    return () => window.clearInterval(intervalId);
  }, [refreshActiveWindow]);

  const visibleRules = useMemo(() => {
    const filtered = filterStyleRules(rules, searchQuery);
    return sortStyleRules(filtered, sort);
  }, [rules, searchQuery, sort]);

  const openModal = (defaults: Partial<ModalDefaults> = {}) => {
    setModalError(null);
    setModalDefaults({ ...DEFAULT_MODAL, ...defaults });
    setModalOpen(true);
  };

  const handleSubmit = async (
    pattern: string,
    matchType: AppContextMatchType,
    tone: ToneProfile,
  ) => {
    setModalError(null);
    const inserted = await addRule(pattern, matchType, tone);
    if (!inserted) {
      setModalError("Impossible d'ajouter cette règle.");
    }
    return inserted;
  };

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">Style</h1>
        <p className="text-body-sm text-charcoal">
          Adaptez le ton de vos dictées selon l&apos;application active — formel
          pour Outlook, décontracté pour Slack, technique pour l&apos;IDE. Actif
          uniquement avec l&apos;auto-édition IA.
        </p>
      </header>

      <ActiveWindowCard
        activeWindow={activeWindow}
        rules={rules}
        busy={busy}
        onRefresh={() => void refreshActiveWindow()}
        onCreateFromActive={() => {
          if (!activeWindow) return;
          openModal({
            pattern: activeWindow.exeName,
            matchType: "exe",
          });
        }}
      />

      <div className="flex flex-wrap items-center justify-between gap-3">
        <Button
          type="button"
          variant="primary"
          className="inline-flex items-center gap-1.5"
          disabled={!loaded || busy}
          onClick={() => openModal()}
        >
          <Plus size={16} aria-hidden />
          Nouvelle règle
        </Button>

        {loaded && rules.length > 0 && (
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
              label={STYLE_SORT_LABELS[sort]}
              disabled={busy}
              onClick={() => setSort((current) => nextStyleSort(current))}
              onMenuSelect={setSort}
              menuTitle="Tri"
              menuOptions={toolbarMenuOptions(
                STYLE_SORT_LABELS,
                STYLE_SORT_ORDER,
              )}
              activeMenuValue={sort}
            >
              <ArrowDownUp size={16} strokeWidth={1.75} />
            </SnippetListToolbarButton>
            <SnippetListToolbarButton
              label="Actualiser la liste"
              disabled={busy}
              onClick={() => {
                void Promise.all([reload(), refreshActiveWindow()]);
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

      {loaded && rules.length > 0 && searchOpen && (
        <TextInput
          label="Rechercher une règle"
          value={searchQuery}
          onChange={(event) => setSearchQuery(event.target.value)}
          placeholder="Motif, ton ou type…"
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

      {loaded && rules.length === 0 && (
        <SectionGlow glow="orange">
          <div className="rounded-lg border border-hairline-strong bg-surface-card p-6 sm:p-8">
            <p className="text-body-md m-0 text-charcoal">
              Aucune règle configurée. Créez une association application →
              style pour personnaliser vos dictées.
            </p>
            <Button
              type="button"
              variant="primary"
              className="mt-4"
              disabled={busy}
              onClick={() => openModal()}
            >
              Créer une règle
            </Button>
          </div>
        </SectionGlow>
      )}

      {loaded && rules.length > 0 && (
        <>
          {visibleRules.length === 0 ? (
            <div className="rounded-lg border border-hairline-strong bg-surface-card px-4 py-8 text-center">
              <p className="text-body-sm m-0 text-charcoal">
                Aucun résultat pour « {searchQuery.trim()} ».
              </p>
            </div>
          ) : (
            <StyleRulesTable
              rules={visibleRules}
              busy={busy}
              onDelete={(id) => {
                void removeRule(id);
              }}
            />
          )}
        </>
      )}

      <StyleRuleModal
        open={modalOpen}
        onClose={() => setModalOpen(false)}
        busy={busy}
        errorMessage={modalError}
        initialPattern={modalDefaults.pattern}
        initialMatchType={modalDefaults.matchType}
        initialTone={modalDefaults.tone}
        onSubmit={handleSubmit}
      />
    </div>
  );
}
