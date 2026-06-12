import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { UiLanguageCode } from "../../i18n/locale";
import { useSettings } from "../../hooks/useSettings";
import { translateError } from "../../lib/translateError";
import { Stagger } from "../motion/Stagger";
import { Button } from "../ui/Button";
import { Kbd } from "../ui/Kbd";
import { ProgressBar } from "../ui/ProgressBar";
import { Select } from "../ui/Select";
import { TextInput } from "../ui/TextInput";
import { AudioInputPanel } from "./AudioInputPanel";
import { ModelsSettingsPanel } from "./ModelsSettingsPanel";
import { SettingsSection } from "./SettingsSection";
import { SettingsToggleRow } from "./SettingsToggleRow";
import {
  captureHotkey,
  formatHotkeyLabel,
  getSettingsSections,
  hotkeyPartLabel,
  hotkeyParts,
  type SettingsSectionId,
} from "./settingsUtils";

export function SettingsView() {
  const { t } = useTranslation();
  const [recordingHotkey, setRecordingHotkey] = useState(false);
  const [pendingHotkey, setPendingHotkey] = useState<string | null>(null);
  const [hotkeyRestoreError, setHotkeyRestoreError] = useState<string | null>(
    null,
  );
  const {
    settings,
    loaded,
    saving,
    errorMessage,
    llmProgress,
    llmProgressModel,
    sttProgress,
    sttProgressModel,
    modelsStatus,
    inferenceInfo,
    autostartEnabled,
    setAutoEdit,
    setAutoLearn,
    setAutoUpdate,
    setSttLanguage,
    setInputDevice,
    setUiLanguage,
    setWhisperModel,
    setLlmModel,
    setInferenceBackend,
    setLowPowerMode,
    setAdaptivePerf,
    setHotkey,
    setAutostart,
    resetSettings,
  } = useSettings();

  const settingsSections = useMemo(() => getSettingsSections(t), [t]);
  const sectionMeta = useMemo(
    () =>
      Object.fromEntries(
        settingsSections.map((section) => [section.id, section]),
      ) as Record<
        SettingsSectionId,
        (typeof settingsSections)[number]
      >,
    [settingsSections],
  );

  const handleHotkeyCapture = useCallback(
    (event: KeyboardEvent) => {
      void (async () => {
        const captured = captureHotkey(event);
        if (captured === null) {
          setPendingHotkey(null);
          setRecordingHotkey(false);
          return;
        }
        setPendingHotkey(captured);
        try {
          await setHotkey(captured);
        } finally {
          setRecordingHotkey(false);
        }
      })();
    },
    [setHotkey],
  );

  const restoreGlobalHotkey = useCallback(async () => {
    try {
      await invoke("set_hotkey_capture_active", { active: false });
      setHotkeyRestoreError(null);
    } catch (err) {
      setHotkeyRestoreError(String(err));
      throw err;
    }
  }, []);

  useEffect(() => {
    if (!recordingHotkey) return;

    let cancelled = false;
    let listening = false;

    void (async () => {
      try {
        await invoke("set_hotkey_capture_active", { active: true });
        if (cancelled) {
          await invoke("set_hotkey_capture_active", { active: false }).catch(
            (err) => {
              console.error("failed to restore hotkey after cancelled capture:", err);
            },
          );
          return;
        }
        window.addEventListener("keydown", handleHotkeyCapture, true);
        listening = true;
      } catch (err) {
        if (!cancelled) {
          setHotkeyRestoreError(String(err));
          setRecordingHotkey(false);
        }
      }
    })();

    return () => {
      cancelled = true;
      if (listening) {
        window.removeEventListener("keydown", handleHotkeyCapture, true);
      }
      void restoreGlobalHotkey();
    };
  }, [recordingHotkey, handleHotkeyCapture, restoreGlobalHotkey]);

  const displayHotkey = pendingHotkey ?? settings.hotkey;

  return (
    <Stagger className="flex w-full flex-col gap-12 pb-8" itemMotion="fade">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">{t("settings.title")}</h1>
        <p className="text-body-sm text-charcoal">{t("settings.subtitle")}</p>
      </header>

          <SettingsSection
            id="general"
            title={sectionMeta.general.label}
            description={sectionMeta.general.description}
            glow="blue"
          >
            <Select
              id="ui-language"
              label={t("settings.uiLanguage.label")}
              value={settings.uiLanguage}
              disabled={!loaded || saving}
              options={[
                { value: "fr", label: t("settings.uiLanguage.fr") },
                { value: "en", label: t("settings.uiLanguage.en") },
              ]}
              onChange={(value) => {
                if (value === "fr" || value === "en") {
                  void setUiLanguage(value as UiLanguageCode);
                }
              }}
            />

            <Select
              id="language"
              label={t("settings.sttLanguage.label")}
              value={settings.sttLanguage}
              disabled={!loaded || saving}
              options={[
                { value: "fr", label: t("settings.sttLanguage.fr") },
                { value: "en", label: t("settings.sttLanguage.en") },
                { value: "auto", label: t("settings.sttLanguage.auto") },
              ]}
              onChange={(value) => {
                void setSttLanguage(value);
              }}
            />

            <AudioInputPanel
              value={settings.inputDevice}
              disabled={!loaded || saving}
              onChange={(deviceId) => {
                void setInputDevice(deviceId);
              }}
            />

            <SettingsToggleRow
              label={t("settings.autoEdit.label")}
              description={
                settings.autoEdit
                  ? t("settings.autoEdit.descriptionOn")
                  : t("settings.autoEdit.descriptionOff")
              }
              checked={settings.autoEdit}
              disabled={!loaded}
              onCheckedChange={(checked) => {
                void setAutoEdit(checked);
              }}
            />

            {settings.autoEdit && llmProgress !== null && (
              <ProgressBar
                value={llmProgress}
                label={t("settings.modelsPanel.downloadLlm", {
                  model: llmProgressModel ?? settings.llmModel,
                })}
              />
            )}

            <SettingsToggleRow
              label={t("settings.autoLearn.label")}
              description={
                settings.autoLearn
                  ? t("settings.autoLearn.descriptionOn")
                  : t("settings.autoLearn.descriptionOff")
              }
              checked={settings.autoLearn}
              disabled={!loaded}
              onCheckedChange={(checked) => {
                void setAutoLearn(checked);
              }}
            />
          </SettingsSection>

          <SettingsSection
            id="models"
            title={sectionMeta.models.label}
            description={sectionMeta.models.description}
            glow="green"
          >
            <ModelsSettingsPanel
              whisperModel={settings.whisperModel}
              llmModel={settings.llmModel}
              sttProgress={sttProgress}
              llmProgress={llmProgress}
              sttProgressModel={sttProgressModel}
              llmProgressModel={llmProgressModel}
              inferenceInfo={inferenceInfo}
              modelsStatus={modelsStatus}
              disabled={!loaded || saving}
              onWhisperChange={(value) => {
                if (
                  value === "auto" ||
                  value === "small" ||
                  value === "distil-fr-dec16"
                ) {
                  void setWhisperModel(value);
                }
              }}
              onLlmChange={(value) => {
                if (
                  value === "auto" ||
                  value === "qwen3-0.6b" ||
                  value === "qwen3-1.7b" ||
                  value === "qwen3-4b"
                ) {
                  void setLlmModel(value);
                }
              }}
            />
          </SettingsSection>

          <SettingsSection
            id="shortcuts"
            title={sectionMeta.shortcuts.label}
            description={sectionMeta.shortcuts.description}
            glow="orange"
          >
            <div className="flex flex-col gap-3">
              <TextInput
                label={t("settings.shortcutsPanel.globalLabel")}
                value={formatHotkeyLabel(displayHotkey, t)}
                readOnly
              />
              <p className="text-body-sm text-charcoal">
                {t("keys.hotkeyPreview")}{" "}
                {hotkeyParts(displayHotkey).map((part, index) => (
                  <span key={`${part}-${index}`}>
                    {index > 0 && ` ${t("common.plusSeparator")} `}
                    <Kbd>{hotkeyPartLabel(t, part)}</Kbd>
                  </span>
                ))}
              </p>
              <Button
                variant={recordingHotkey ? "primary" : "ghost"}
                disabled={saving}
                onClick={() => {
                  setRecordingHotkey((current) => !current);
                  setPendingHotkey(null);
                }}
              >
                {recordingHotkey
                  ? t("keys.hotkeyCapturePrompt", {
                      escapeHint: t("keys.escapeHint"),
                    })
                  : t("keys.hotkeyEdit")}
              </Button>
            </div>

            {hotkeyRestoreError && (
              <div className="space-y-2">
                <p className="text-body-sm text-accent-red">
                  {translateError(hotkeyRestoreError, t)}
                </p>
                <Button
                  variant="ghost"
                  onClick={() => {
                    void restoreGlobalHotkey();
                  }}
                >
                  {t("keys.hotkeyRestore")}
                </Button>
              </div>
            )}
          </SettingsSection>

          <SettingsSection
            id="advanced"
            title={sectionMeta.advanced.label}
            description={sectionMeta.advanced.description}
            glow="blue"
          >
            <SettingsToggleRow
              label={t("settings.autoUpdate.label")}
              description={
                settings.autoUpdate
                  ? t("settings.autoUpdate.descriptionOn")
                  : t("settings.autoUpdate.descriptionOff")
              }
              checked={settings.autoUpdate}
              disabled={!loaded}
              onCheckedChange={(checked) => {
                void setAutoUpdate(checked);
              }}
            />

            <SettingsToggleRow
              label={t("settings.autostart.label")}
              checked={autostartEnabled}
              disabled={!loaded}
              onCheckedChange={(checked) => {
                void setAutostart(checked);
              }}
            />

            <SettingsToggleRow
              label={t("settings.lowPower.label")}
              description={
                settings.lowPowerMode
                  ? t("settings.lowPower.descriptionOn")
                  : t("settings.lowPower.descriptionOff")
              }
              checked={settings.lowPowerMode}
              disabled={!loaded}
              onCheckedChange={(checked) => {
                void setLowPowerMode(checked);
              }}
            />

            <SettingsToggleRow
              label={t("settings.adaptivePerf.label")}
              description={
                settings.adaptivePerf
                  ? t("settings.adaptivePerf.descriptionOn")
                  : t("settings.adaptivePerf.descriptionOff")
              }
              checked={settings.adaptivePerf}
              disabled={!loaded}
              onCheckedChange={(checked) => {
                void setAdaptivePerf(checked);
              }}
            />

            <Select
              id="inference-backend"
              label={t("settings.inferenceBackend.label")}
              value={settings.inferenceBackend}
              disabled={!loaded || saving}
              options={[
                {
                  value: "auto",
                  label: t("settings.inferenceBackend.auto"),
                },
                { value: "cpu", label: t("settings.inferenceBackend.cpu") },
              ]}
              onChange={(value) => {
                void setInferenceBackend(value);
              }}
            />

            {inferenceInfo && (
              <p className="text-caption text-ash">
                {t("settings.inferenceInfo", {
                  compiled: inferenceInfo.compiled_backend,
                  active: inferenceInfo.active_backend,
                  tier: inferenceInfo.perf_tier,
                  totalRam: inferenceInfo.total_ram_gb.toFixed(0),
                  freeRam: inferenceInfo.avail_ram_gb.toFixed(1),
                  whisper: inferenceInfo.effective_whisper,
                  llm: inferenceInfo.effective_llm,
                  chunk: inferenceInfo.vad_chunk_size,
                })}
              </p>
            )}
          </SettingsSection>

          {errorMessage && (
            <p className="text-body-sm text-accent-red">
              {translateError(errorMessage, t)}
            </p>
          )}

          <footer className="flex flex-wrap items-center justify-between gap-4 border-t border-divider-soft pt-6">
            <Button
              variant="ghost"
              disabled={!loaded || saving}
              onClick={() => {
                void resetSettings();
              }}
            >
              {t("common.reset")}
            </Button>
            <Button variant="primary" disabled>
              {saving ? t("common.saving") : t("common.autoSave")}
            </Button>
          </footer>
    </Stagger>
  );
}
