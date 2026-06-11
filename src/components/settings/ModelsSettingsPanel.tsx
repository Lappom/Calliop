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
  const whisperOptions = buildWhisperSelectOptions(modelsStatus?.whisper);
  const llmOptions = buildLlmSelectOptions(modelsStatus?.llm);

  return (
    <div className="space-y-6">
      <Select
        id="whisper-model"
        label="Modèle Whisper (STT)"
        value={whisperModel}
        options={whisperOptions}
        disabled={disabled}
        onChange={onWhisperChange}
      />

      {sttProgress !== null && (
        <ProgressBar
          value={sttProgress}
          label={`Téléchargement Whisper (${sttProgressModel ?? whisperModel})`}
        />
      )}

      <Select
        id="llm-model"
        label="Modèle LLM (auto-edits)"
        value={llmModel}
        options={llmOptions}
        disabled={disabled}
        onChange={onLlmChange}
      />

      {llmProgress !== null && (
        <ProgressBar
          value={llmProgress}
          label={`Téléchargement LLM (${llmProgressModel ?? llmModel})`}
        />
      )}

      {inferenceInfo && (
        <p className="text-caption text-ash">
          Backend {inferenceInfo.active_backend.toUpperCase()}
          {inferenceInfo.gpu_available ? " · GPU Vulkan" : " · CPU"}
          {" · "}
          profil {inferenceInfo.perf_tier}
          {" · "}
          effectif {inferenceInfo.effective_whisper} / {inferenceInfo.effective_llm}
        </p>
      )}
    </div>
  );
}
