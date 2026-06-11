import {
  ArrowDownUp,
  Plus,
  RefreshCw,
  Search,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useUiLocale } from "../../i18n/useUiLocale";
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
  getStyleSortLabels,
  nextStyleSort,
  sortStyleRules,
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
  const { t, intlLocale } = useUiLocale();
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

  const sortLabels = useMemo(() => getStyleSortLabels(t), [t]);

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      void refreshActiveWindow();
    }, 1000);
    return () => window.clearInterval(intervalId);
  }, [refreshActiveWindow]);

  const visibleRules = useMemo(() => {
    const filtered = filterStyleRules(rules, searchQuery, t);
    return sortStyleRules(filtered, sort, t, intlLocale);
  }, [rules, searchQuery, sort, t, intlLocale]);

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
      setModalError(t("style.errors.cannotAdd"));
    }
    return inserted;
  };

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">{t("style.title")}</h1>
        <p className="text-body-sm text-charcoal">{t("style.subtitle")}</p>
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
          {t("style.newRule")}
        </Button>

        {loaded && rules.length > 0 && (
          <div className="flex items-center gap-1">
            <SnippetListToolbarButton
              label={t("common.search")}
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
              label={sortLabels[sort]}
              disabled={busy}
              onClick={() => setSort((current) => nextStyleSort(current))}
              onMenuSelect={setSort}
              menuTitle={t("common.sort")}
              menuOptions={toolbarMenuOptions(sortLabels, STYLE_SORT_ORDER)}
              activeMenuValue={sort}
            >
              <ArrowDownUp size={16} strokeWidth={1.75} />
            </SnippetListToolbarButton>
            <SnippetListToolbarButton
              label={t("common.refreshList")}
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
          label={t("style.searchLabel")}
          value={searchQuery}
          onChange={(event) => setSearchQuery(event.target.value)}
          placeholder={t("style.searchPlaceholder")}
          disabled={busy}
          autoFocus
        />
      )}

      {errorMessage && (
        <p className="text-body-sm text-accent-red">{errorMessage}</p>
      )}

      {!loaded && (
        <p className="text-body-sm text-charcoal">{t("common.loading")}</p>
      )}

      {loaded && rules.length === 0 && (
        <SectionGlow glow="orange">
          <div className="rounded-lg border border-hairline-strong bg-surface-card p-6 sm:p-8">
            <p className="text-body-md m-0 text-charcoal">{t("style.empty")}</p>
            <Button
              type="button"
              variant="primary"
              className="mt-4"
              disabled={busy}
              onClick={() => openModal()}
            >
              {t("style.createRule")}
            </Button>
          </div>
        </SectionGlow>
      )}

      {loaded && rules.length > 0 && (
        <>
          {visibleRules.length === 0 ? (
            <div className="rounded-lg border border-hairline-strong bg-surface-card px-4 py-8 text-center">
              <p className="text-body-sm m-0 text-charcoal">
                {t("common.noResults", { query: searchQuery.trim() })}
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
