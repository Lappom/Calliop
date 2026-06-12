import { RefreshCw, Trash2 } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useTranslation } from "react-i18next";
import type { ModelStatusEntry } from "../../hooks/useSettings";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import { Button } from "../ui/Button";
import { getModelStatusLabel, type ModelInstallStatus } from "./modelCatalog";

interface ModelManageActionsProps {
  kind: "whisper" | "llm";
  modelId: string;
  entry: ModelStatusEntry | undefined;
  formatBytes: (bytes: number | null) => string;
  disabled: boolean;
  busyAction: "delete" | "reinstall" | null;
  onDelete: () => Promise<void>;
  onReinstall: () => Promise<void>;
}

function resolveInstallStatus(
  entry: ModelStatusEntry | undefined,
): ModelInstallStatus {
  if (!entry?.installed) return "missing";
  if (entry.active) return "active";
  return "installed";
}

export function ModelManageActions({
  kind,
  modelId,
  entry,
  formatBytes,
  disabled,
  busyAction,
  onDelete,
  onReinstall,
}: ModelManageActionsProps) {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();

  if (modelId === "auto" || !entry?.installed) {
    return null;
  }

  const status = resolveInstallStatus(entry);
  const statusLabel = getModelStatusLabel(status, t);
  const isActive = status === "active";
  const isBusy = busyAction !== null;
  const motionProps = reducedMotion
    ? {}
    : {
        initial: { opacity: 0, y: -6 },
        animate: { opacity: 1, y: 0 },
        exit: { opacity: 0, y: -4 },
        transition: { duration: 0.15, ease: [0.22, 1, 0.36, 1] as const },
      };

  return (
    <AnimatePresence mode="wait">
      <motion.div
        key={`${kind}-${modelId}`}
        {...motionProps}
        className="flex flex-col gap-3 rounded-lg border border-hairline bg-surface-card px-3.5 py-3 sm:flex-row sm:items-center sm:justify-between"
      >
        <div className="min-w-0">
          <p className="text-body-sm text-ink">
            {t("settings.modelsPanel.manage.diskSize", {
              size: formatBytes(entry.size_bytes),
            })}
          </p>
          <p
            className={[
              "text-caption",
              status === "active" ? "text-accent-green" : "text-charcoal",
            ].join(" ")}
          >
            {statusLabel}
          </p>
          {isActive && (
            <p className="mt-1 text-caption text-ash">
              {t("settings.modelsPanel.manage.deleteActiveHint")}
            </p>
          )}
        </div>

        <div className="flex shrink-0 flex-wrap items-center gap-2">
          <Button
            type="button"
            variant="outline"
            disabled={disabled || isBusy}
            className="h-8 gap-1.5 px-3 text-xs"
            onClick={() => {
              void onReinstall();
            }}
          >
            <RefreshCw
              size={14}
              strokeWidth={2}
              className={[
                "shrink-0",
                busyAction === "reinstall" ? "animate-spin" : "",
              ].join(" ")}
              aria-hidden
            />
            {busyAction === "reinstall"
              ? t("settings.modelsPanel.manage.reinstalling")
              : t("settings.modelsPanel.manage.reinstall")}
          </Button>
          <Button
            type="button"
            variant="ghost"
            disabled={disabled || isBusy || isActive}
            title={
              isActive
                ? t("settings.modelsPanel.manage.deleteActiveHint")
                : undefined
            }
            className="h-8 gap-1.5 px-3 text-xs"
            onClick={() => {
              void onDelete();
            }}
          >
            <Trash2 size={14} strokeWidth={2} className="shrink-0" aria-hidden />
            {busyAction === "delete"
              ? t("settings.modelsPanel.manage.deleting")
              : t("settings.modelsPanel.manage.delete")}
          </Button>
        </div>
      </motion.div>
    </AnimatePresence>
  );
}
