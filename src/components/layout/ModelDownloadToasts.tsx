import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useModelDownloads } from "../../hooks/useModelDownloads";
import { getModelDownloadLabels } from "../../lib/modelLabels";
import { ProgressBar } from "../ui/ProgressBar";

export function ModelDownloadToasts() {
  const { t } = useTranslation();
  const downloads = useModelDownloads();
  const modelLabels = useMemo(() => getModelDownloadLabels(t), [t]);

  if (downloads.length === 0) {
    return null;
  }

  return (
    <div
      className="pointer-events-none fixed bottom-4 right-4 z-[60] flex w-[min(100vw-2rem,320px)] flex-col gap-3"
      aria-live="polite"
      aria-label={t("window.downloadToasts.aria")}
    >
      {downloads.map((download) => (
        <div
          key={download.kind}
          className="animate-toast-in pointer-events-auto rounded-lg border border-hairline-strong bg-surface-elevated p-4"
          role="status"
        >
          <p className="text-body-sm m-0 mb-3 font-medium text-ink">
            {modelLabels.formatTitle(download.kind)}
          </p>
          <ProgressBar
            value={download.percent}
            label={modelLabels.formatLabel(download.modelId)}
          />
        </div>
      ))}
    </div>
  );
}
