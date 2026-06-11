import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { AppWindow, RefreshCw, Sparkles } from "lucide-react";
import type { ActiveWindow, AppContextRule } from "../../hooks/useAppContext";
import { SectionGlow } from "../layout/SectionGlow";
import { Button } from "../ui/Button";
import { getToneMeta, resolveActiveTone } from "./styleUtils";
import { ToneBadge } from "./ToneBadge";

interface ActiveWindowCardProps {
  activeWindow: ActiveWindow | null;
  rules: AppContextRule[];
  busy: boolean;
  onRefresh: () => void;
  onCreateFromActive: () => void;
}

export function ActiveWindowCard({
  activeWindow,
  rules,
  busy,
  onRefresh,
  onCreateFromActive,
}: ActiveWindowCardProps) {
  const { t } = useTranslation();
  const toneMeta = useMemo(() => getToneMeta(t), [t]);
  const activeTone = resolveActiveTone(rules, activeWindow);

  return (
    <SectionGlow glow="blue">
      <div className="rounded-lg border border-hairline-strong bg-surface-card p-4 sm:p-6">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div className="flex min-w-0 gap-4">
            <div className="flex size-11 shrink-0 items-center justify-center rounded-lg border border-hairline-strong bg-surface-elevated text-ink">
              <AppWindow size={20} strokeWidth={1.5} />
            </div>
            <div className="min-w-0">
              <p className="text-caption mb-1 text-charcoal">
                {t("style.activeWindow.label")}
              </p>
              {activeWindow ? (
                <>
                  <p className="text-body-md m-0 truncate font-medium text-ink">
                    {activeWindow.title || t("common.withoutTitle")}
                  </p>
                  <p className="text-caption mt-1 truncate text-ash">
                    {activeWindow.exeName}
                  </p>
                </>
              ) : (
                <p className="text-body-sm m-0 text-ash">
                  {t("style.activeWindow.notDetected")}
                </p>
              )}
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2 sm:flex-col sm:items-end">
            <div className="flex items-center gap-2">
              <Sparkles size={14} className="text-accent-blue" aria-hidden />
              <ToneBadge tone={activeTone} />
            </div>
            <p className="text-caption m-0 max-w-xs text-right text-ash">
              {toneMeta[activeTone].description}
            </p>
          </div>
        </div>

        <div className="mt-5 flex flex-wrap gap-2 border-t border-divider-soft pt-4">
          <Button
            type="button"
            variant="ghost"
            disabled={busy}
            className="inline-flex items-center gap-1.5"
            onClick={onRefresh}
          >
            <RefreshCw size={14} aria-hidden />
            {t("style.activeWindow.refresh")}
          </Button>
          {activeWindow && (
            <Button
              type="button"
              variant="outline"
              disabled={busy}
              onClick={onCreateFromActive}
            >
              {t("style.activeWindow.createRule")}
            </Button>
          )}
        </div>
      </div>
    </SectionGlow>
  );
}
