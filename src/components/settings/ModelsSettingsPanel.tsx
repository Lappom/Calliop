import { useTranslation } from "react-i18next";
import type { InferenceInfo, ModelsStatus } from "../../hooks/useSettings";
import { ProgressBar } from "../ui/ProgressBar";
import { Select } from "../ui/Select";
import { buildLlmSelectOptions, buildWhisperSelectOptions } from "./modelCatalog";

interface ModelsSettingsPanelProps {
  whisperModel: string;
  llmModel: string;
  sttProgress: number | null;
  llmProgress: number | null;
  sttProgressModel: string | null;
  llmProgressModel: string | null;
  inferenceInfo: InferenceInfo | null;
  modelsStatus: ModelsStatus | null;
  disabled: boolean;
  onWhisperChange: (value: string) => void;
  onLlmChange: (value: string) => void;
}

export function ModelsSettingsPanel({
  whisperModel,
  llmModel,
  sttProgress,
  llmProgress,
  sttProgressModel,
  llmProgressModel,
  inferenceInfo,
  modelsStatus,
  disabled,
  onWhisperChange,
  onLlmChange,
}: ModelsSettingsPanelProps) {
  const { t } = useTranslation();
  const whisperOptions = buildWhisperSelectOptions(modelsStatus?.whisper, t);
  const llmOptions = buildLlmSelectOptions(modelsStatus?.llm, t);

  return (
    <div className="space-y-6">
      <Select
        id="whisper-model"
        label={t("settings.modelsPanel.whisperLabel")}
        value={whisperModel}
        options={whisperOptions}
        disabled={disabled}
        onChange={onWhisperChange}
      />

      {sttProgress !== null && (
        <ProgressBar
          value={sttProgress}
          label={t("settings.modelsPanel.downloadWhisper", {
            model: sttProgressModel ?? whisperModel,
          })}
        />
      )}

      <Select
        id="llm-model"
        label={t("settings.modelsPanel.llmLabel")}
        value={llmModel}
        options={llmOptions}
        disabled={disabled}
        onChange={onLlmChange}
      />

      {llmProgress !== null && (
        <ProgressBar
          value={llmProgress}
          label={t("settings.modelsPanel.downloadLlm", {
            model: llmProgressModel ?? llmModel,
          })}
        />
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
