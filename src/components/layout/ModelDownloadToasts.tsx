import { AnimatePresence, motion } from "motion/react";
import { memo, useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ActiveModelDownload } from "../../hooks/useModelDownloads";
import { useModelDownloads } from "../../hooks/useModelDownloads";
import { getModelDownloadLabels } from "../../lib/modelLabels";
import { pickVariants, toastVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import { ProgressBar } from "../ui/ProgressBar";

const DOWNLOAD_STACK_ORDER = { whisper: 0, llm: 1 } as const;

function sortDownloads(downloads: ActiveModelDownload[]) {
  return [...downloads].sort(
    (a, b) => DOWNLOAD_STACK_ORDER[a.kind] - DOWNLOAD_STACK_ORDER[b.kind],
  );
}

interface ModelDownloadToastItemProps {
  download: ActiveModelDownload;
  title: string;
  label: string;
  variants: ReturnType<typeof pickVariants>;
}

const ModelDownloadToastItem = memo(function ModelDownloadToastItem({
  download,
  title,
  label,
  variants,
}: ModelDownloadToastItemProps) {
  return (
    <motion.div
      layout={false}
      variants={variants}
      initial="initial"
      animate="animate"
      exit="exit"
      className="pointer-events-auto w-full shrink-0 rounded-lg border border-hairline-strong bg-surface-elevated p-4"
      role="status"
    >
      <p className="text-body-sm m-0 mb-3 font-medium text-ink">{title}</p>
      <ProgressBar value={download.percent} label={label} />
    </motion.div>
  );
});

export function ModelDownloadToasts() {
  const { t } = useTranslation();
  const downloads = useModelDownloads();
  const reducedMotion = useReducedMotion();
  const variants = pickVariants(toastVariants, reducedMotion);
  const modelLabels = useMemo(() => getModelDownloadLabels(t), [t]);
  const sortedDownloads = useMemo(() => sortDownloads(downloads), [downloads]);

  if (sortedDownloads.length === 0) {
    return null;
  }

  return (
    <div
      className="pointer-events-none fixed bottom-4 right-4 z-[60] flex w-[min(100vw-2rem,320px)] flex-col items-stretch gap-3"
      aria-live="polite"
      aria-label={t("window.downloadToasts.aria")}
    >
      <AnimatePresence initial={false} mode="sync">
        {sortedDownloads.map((download) => (
          <ModelDownloadToastItem
            key={download.kind}
            download={download}
            title={modelLabels.formatTitle(download.kind)}
            label={modelLabels.formatLabel(download.modelId)}
            variants={variants}
          />
        ))}
      </AnimatePresence>
    </div>
  );
}
