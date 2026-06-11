import { AppWindow, RefreshCw, Sparkles } from "lucide-react";
import type { ActiveWindow, AppContextRule } from "../../hooks/useAppContext";
import { SectionGlow } from "../layout/SectionGlow";
import { BadgePill } from "../ui/BadgePill";
import { Button } from "../ui/Button";
import { resolveActiveTone, TONE_META } from "./styleUtils";

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
  const activeTone = resolveActiveTone(rules, activeWindow);
  const toneMeta = TONE_META[activeTone];

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
                Application au premier plan
              </p>
              {activeWindow ? (
                <>
                  <p className="text-body-md m-0 truncate font-medium text-ink">
                    {activeWindow.title || "(sans titre)"}
                  </p>
                  <p className="text-caption mt-1 truncate text-ash">
                    {activeWindow.exeName}
                  </p>
                </>
              ) : (
                <p className="text-body-sm m-0 text-ash">
                  Aucune fenêtre détectée — ou Calliop est au premier plan.
                </p>
              )}
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2 sm:flex-col sm:items-end">
            <div className="flex items-center gap-2">
              <Sparkles size={14} className="text-accent-blue" aria-hidden />
              <BadgePill active>{toneMeta.label}</BadgePill>
            </div>
            <p className="text-caption m-0 max-w-xs text-right text-ash">
              {toneMeta.description}
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
            Actualiser
          </Button>
          {activeWindow && (
            <Button
              type="button"
              variant="outline"
              disabled={busy}
              onClick={onCreateFromActive}
            >
              Règle pour cette app
            </Button>
          )}
        </div>
      </div>
    </SectionGlow>
  );
}
