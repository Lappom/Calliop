import { RefreshCw, Trash2 } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useTranslation } from "react-i18next";
import type { ModelStatusEntry } from "../../hooks/useSettings";
import { listRowVariants, pickVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import { Button } from "../ui/Button";
import {
  getModelLabel,
  getModelStatusLabel,
  listInstalledModels,
  type ModelInstallStatus,
} from "./modelCatalog";

export type ModelBusyKey =
  `${"whisper" | "llm"}:${string}:${"delete" | "reinstall"}`;

export function makeModelBusyKey(
  kind: "whisper" | "llm",
  modelId: string,
  action: "delete" | "reinstall",
): ModelBusyKey {
  return `${kind}:${modelId}:${action}`;
}

export function resolveRowBusyAction(
  busyKey: ModelBusyKey | null,
  kind: "whisper" | "llm",
  modelId: string,
): "delete" | "reinstall" | null {
  if (busyKey === makeModelBusyKey(kind, modelId, "delete")) return "delete";
  if (busyKey === makeModelBusyKey(kind, modelId, "reinstall")) {
    return "reinstall";
  }
  return null;
}

interface InstalledModelsListProps {
  kind: "whisper" | "llm";
  entries: ModelStatusEntry[] | undefined;
  formatBytes: (bytes: number | null) => string;
  disabled: boolean;
  busyKey: ModelBusyKey | null;
  onDelete: (modelId: string) => Promise<void>;
  onReinstall: (modelId: string) => Promise<void>;
}

function resolveInstallStatus(entry: ModelStatusEntry): ModelInstallStatus {
  if (entry.active) return "active";
  return "installed";
}

interface InstalledModelRowProps {
  kind: "whisper" | "llm";
  entry: ModelStatusEntry;
  index: number;
  formatBytes: (bytes: number | null) => string;
  disabled: boolean;
  busyKey: ModelBusyKey | null;
  onDelete: () => Promise<void>;
  onReinstall: () => Promise<void>;
}

function InstalledModelRow({
  kind,
  entry,
  index,
  formatBytes,
  disabled,
  busyKey,
  onDelete,
  onReinstall,
}: InstalledModelRowProps) {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const variants = pickVariants(listRowVariants, reducedMotion);
  const busyAction = resolveRowBusyAction(busyKey, kind, entry.id);
  const rowDisabled =
    disabled ||
    (busyKey !== null && busyAction === null);
  const status = resolveInstallStatus(entry);
  const statusLabel = getModelStatusLabel(status, t);
  const isActive = status === "active";

  return (
    <motion.li
      custom={index}
      variants={variants}
      initial="initial"
      animate="animate"
      exit="exit"
      className="border-b border-divider-soft last:border-b-0"
    >
      <div className="flex flex-col gap-3 px-3.5 py-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0">
          <p className="text-body-sm text-ink">{getModelLabel(kind, entry.id, t)}</p>
          <p className="text-caption text-charcoal">
            {t("settings.modelsPanel.manage.diskSize", {
              size: formatBytes(entry.size_bytes),
            })}
          </p>
          <p
            className={[
              "text-caption",
              isActive ? "text-accent-green" : "text-charcoal",
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
            disabled={rowDisabled}
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
            disabled={rowDisabled || isActive}
            title={
              isActive
                ? t("settings.modelsPanel.manage.deleteActiveHint")
                : undefined
            }
            className="h-8 gap-1.5 px-3 text-xs hover:text-accent-red"
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
      </div>
    </motion.li>
  );
}

export function InstalledModelsList({
  kind,
  entries,
  formatBytes,
  disabled,
  busyKey,
  onDelete,
  onReinstall,
}: InstalledModelsListProps) {
  const { t } = useTranslation();
  const installed = listInstalledModels(entries);

  if (installed.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <p className="text-caption text-ash">
        {t("settings.modelsPanel.manage.installedTitle")}
      </p>
      <div className="overflow-hidden rounded-lg border border-hairline bg-surface-card">
        <ul>
          <AnimatePresence initial={false}>
            {installed.map((entry, index) => (
              <InstalledModelRow
                key={entry.id}
                kind={kind}
                entry={entry}
                index={index}
                formatBytes={formatBytes}
                disabled={disabled}
                busyKey={busyKey}
                onDelete={() => onDelete(entry.id)}
                onReinstall={() => onReinstall(entry.id)}
              />
            ))}
          </AnimatePresence>
        </ul>
      </div>
    </div>
  );
}
