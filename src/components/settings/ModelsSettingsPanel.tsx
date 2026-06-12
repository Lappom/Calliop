import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import type { InferenceInfo, ModelsStatus } from "../../hooks/useSettings";
import { ProgressBar } from "../ui/ProgressBar";
import { Select } from "../ui/Select";
import {
  InstalledModelsList,
  makeModelBusyKey,
  type ModelBusyKey,
} from "./InstalledModelsList";
import { buildLlmSelectOptions, buildWhisperSelectOptions } from "./modelCatalog";

interface ModelsSettingsPanelProps {
  whisperModel: string;
  llmModel: string;
  lowPowerMode: boolean;
  sttProgress: number | null;
  llmProgress: number | null;
  sttProgressModel: string | null;
  llmProgressModel: string | null;
  inferenceInfo: InferenceInfo | null;
  modelsStatus: ModelsStatus | null;
  disabled: boolean;
  formatBytes: (bytes: number | null) => string;
  onWhisperChange: (value: string) => void;
  onLlmChange: (value: string) => void;
  onDeleteModel: (kind: "whisper" | "llm", modelId: string) => Promise<void>;
  onReinstallModel: (kind: "whisper" | "llm", modelId: string) => Promise<void>;
}

export function ModelsSettingsPanel({
  whisperModel,
  llmModel,
  lowPowerMode,
  sttProgress,
  llmProgress,
  sttProgressModel,
  llmProgressModel,
  inferenceInfo,
  modelsStatus,
  disabled,
  formatBytes,
  onWhisperChange,
  onLlmChange,
  onDeleteModel,
  onReinstallModel,
}: ModelsSettingsPanelProps) {
  const { t } = useTranslation();
  const [busyKey, setBusyKey] = useState<ModelBusyKey | null>(null);
  const whisperOptions = buildWhisperSelectOptions(modelsStatus?.whisper, t);
  const llmOptions = buildLlmSelectOptions(modelsStatus?.llm, t);
  const showLowPowerHint =
    lowPowerMode && (sttProgress !== null || llmProgress !== null);

  const runModelAction = useCallback(
    async (key: ModelBusyKey, action: () => Promise<void>) => {
      setBusyKey(key);
      try {
        await action();
      } finally {
        setBusyKey(null);
      }
    },
    [],
  );

  return (
    <div className="space-y-6">
      <div className="space-y-3">
        <Select
          id="whisper-model"
          label={t("settings.modelsPanel.whisperLabel")}
          value={whisperModel}
          options={whisperOptions}
          disabled={disabled || busyKey !== null}
          onChange={onWhisperChange}
        />

        <InstalledModelsList
          kind="whisper"
          entries={modelsStatus?.whisper}
          formatBytes={formatBytes}
          disabled={disabled}
          busyKey={busyKey}
          onDelete={(modelId) =>
            runModelAction(makeModelBusyKey("whisper", modelId, "delete"), () =>
              onDeleteModel("whisper", modelId),
            )
          }
          onReinstall={(modelId) =>
            runModelAction(makeModelBusyKey("whisper", modelId, "reinstall"), () =>
              onReinstallModel("whisper", modelId),
            )
          }
        />
      </div>

      {sttProgress !== null && (
        <ProgressBar
          value={sttProgress}
          label={t("settings.modelsPanel.downloadWhisper", {
            model: sttProgressModel ?? whisperModel,
          })}
        />
      )}

      <div className="space-y-3">
        <Select
          id="llm-model"
          label={t("settings.modelsPanel.llmLabel")}
          value={llmModel}
          options={llmOptions}
          disabled={disabled || busyKey !== null}
          onChange={onLlmChange}
        />

        <InstalledModelsList
          kind="llm"
          entries={modelsStatus?.llm}
          formatBytes={formatBytes}
          disabled={disabled}
          busyKey={busyKey}
          onDelete={(modelId) =>
            runModelAction(makeModelBusyKey("llm", modelId, "delete"), () =>
              onDeleteModel("llm", modelId),
            )
          }
          onReinstall={(modelId) =>
            runModelAction(makeModelBusyKey("llm", modelId, "reinstall"), () =>
              onReinstallModel("llm", modelId),
            )
          }
        />
      </div>

      {llmProgress !== null && (
        <ProgressBar
          value={llmProgress}
          label={t("settings.modelsPanel.downloadLlm", {
            model: llmProgressModel ?? llmModel,
          })}
        />
      )}

      {showLowPowerHint && (
        <p className="text-caption text-ash">
          {t("settings.modelsPanel.lowPowerDownloadHint")}
        </p>
      )}

      {inferenceInfo && (
        <p className="text-caption text-ash">
          {t("settings.modelsPanel.inferenceSummary", {
            backend: inferenceInfo.active_backend.toUpperCase(),
            gpuOrCpu: inferenceInfo.gpu_available
              ? t("settings.modelsPanel.gpuVulkan")
              : t("settings.modelsPanel.cpuOnly"),
            tier: inferenceInfo.perf_tier,
            whisper: inferenceInfo.effective_whisper,
            llm: inferenceInfo.effective_llm,
          })}
        </p>
      )}
    </div>
  );
}
