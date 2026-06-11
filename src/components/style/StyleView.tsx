import {
  ArrowDownUp,
  Plus,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useUiLocale } from "../../i18n/useUiLocale";
import type {
  AppContextMatchType,
  ToneProfile,
} from "../../hooks/useAppContext";
import { useAppContext } from "../../hooks/useAppContext";
import { useRefreshSpin } from "../../hooks/useRefreshSpin";
import { EmptyStateCard } from "../motion/EmptyStateCard";
import { NoResultsCard } from "../motion/NoResultsCard";
import { Stagger } from "../motion/Stagger";
import { SnippetListToolbarButton } from "../snippets/SnippetListToolbarButton";
import { Button } from "../ui/Button";
import { ExpandableSearchField } from "../ui/ExpandableSearchField";
import { RefreshIcon } from "../ui/RefreshIcon";
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

  const { spinning: refreshSpinning, runRefresh } = useRefreshSpin(busy);

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
    <>
    <Stagger className="flex flex-col gap-8" itemMotion="fade">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">{t("style.title")}</h1>
        <p className="text-body-sm text-charcoal">{t("style.subtitle")}</p>
      </header>

      <ActiveWindowCard
        activeWindow={activeWindow}
        rules={rules}
        busy={busy}
        onRefresh={() => refreshActiveWindow()}
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
            <ExpandableSearchField
              open={searchOpen}
              disabled={busy}
              label={t("style.searchLabel")}
              placeholder={t("style.searchPlaceholder")}
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
              disabled={busy || refreshSpinning}
              onClick={() => {
                void runRefresh(async () => {
                  await Promise.all([reload(), refreshActiveWindow()]);
                });
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

      {loaded && rules.length === 0 && (
        <EmptyStateCard glow="orange">
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
        </EmptyStateCard>
      )}

      {loaded && rules.length > 0 && (
        <>
          {visibleRules.length === 0 ? (
            <NoResultsCard>
              <p className="text-body-sm m-0 text-charcoal">
                {t("common.noResults", { query: searchQuery.trim() })}
              </p>
            </NoResultsCard>
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
    </Stagger>

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
    </>
  );
}
