import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useSettings } from "../../hooks/useSettings";
import { Button } from "../ui/Button";
import { Kbd } from "../ui/Kbd";
import { ProgressBar } from "../ui/ProgressBar";
import { Select } from "../ui/Select";
import { TextInput } from "../ui/TextInput";
import { ModelsSettingsPanel } from "./ModelsSettingsPanel";
import { SettingsSection } from "./SettingsSection";
import { SettingsToggleRow } from "./SettingsToggleRow";
import {
  captureHotkey,
  formatHotkeyLabel,
  hotkeyParts,
  SETTINGS_SECTIONS,
  type SettingsSectionId,
} from "./settingsUtils";

export function SettingsView() {
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
    setWhisperModel,
    setLlmModel,
    setInferenceBackend,
    setHotkey,
    setAutostart,
    resetSettings,
  } = useSettings();

  const sectionMeta = useMemo(
    () =>
      Object.fromEntries(
        SETTINGS_SECTIONS.map((section) => [section.id, section]),
      ) as Record<
        SettingsSectionId,
        (typeof SETTINGS_SECTIONS)[number]
      >,
    [],
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
    <div className="w-full">
      <header className="mb-8">
        <h1 className="text-heading-md mb-2 text-ink">Paramètres</h1>
        <p className="text-body-sm text-charcoal">
          Les réglages sont enregistrés localement sur cet appareil.
        </p>
      </header>

      <div className="flex flex-col gap-12 pb-8">
          <SettingsSection
            id="general"
            title={sectionMeta.general.label}
            description={sectionMeta.general.description}
            glow="blue"
          >
            <Select
              id="language"
              label="Langue de dictée"
              value={settings.sttLanguage}
              disabled={!loaded || saving}
              options={[
                { value: "fr", label: "Français" },
                { value: "en", label: "Anglais" },
                { value: "auto", label: "Détection automatique" },
              ]}
              onChange={(value) => {
                void setSttLanguage(value);
              }}
            />

            <SettingsToggleRow
              label="Auto-edits IA"
              description={
                settings.autoEdit
                  ? "Nettoie fillers, ponctuation et reformulation légère via un modèle local."
                  : "Mode verbatim : la transcription brute est injectée sans post-traitement LLM."
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
                label={`Téléchargement LLM (${llmProgressModel ?? settings.llmModel})`}
              />
            )}

            <SettingsToggleRow
              label="Apprentissage automatique des corrections"
              description={
                settings.autoLearn
                  ? "Détecte les corrections dans l'application cible et enrichit le dictionnaire localement."
                  : "Désactivé : seules les corrections manuelles sont prises en compte."
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
                if (value === "small" || value === "distil-fr-dec16") {
                  void setWhisperModel(value);
                }
              }}
              onLlmChange={(value) => {
                if (
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
                label="Raccourci global"
                value={formatHotkeyLabel(displayHotkey)}
                readOnly
              />
              <p className="text-body-sm text-charcoal">
                Aperçu :{" "}
                {hotkeyParts(displayHotkey).map((part, index) => (
                  <span key={`${part}-${index}`}>
                    {index > 0 && " + "}
                    <Kbd>{part === "Space" ? "Espace" : part}</Kbd>
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
                  ? "Appuyez sur la combinaison… (Échap pour annuler)"
                  : "Modifier le raccourci"}
              </Button>
            </div>

            {hotkeyRestoreError && (
              <div className="space-y-2">
                <p className="text-body-sm text-accent-red">
                  {hotkeyRestoreError}
                </p>
                <Button
                  variant="ghost"
                  onClick={() => {
                    void restoreGlobalHotkey();
                  }}
                >
                  Réactiver le raccourci de dictée
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
              label="Mises à jour automatiques"
              description={
                settings.autoUpdate
                  ? "Vérifie les nouvelles versions sur GitHub au démarrage et installe les mises à jour signées."
                  : "Désactivé : aucune vérification réseau pour les mises à jour de l'application."
              }
              checked={settings.autoUpdate}
              disabled={!loaded}
              onCheckedChange={(checked) => {
                void setAutoUpdate(checked);
              }}
            />

            <SettingsToggleRow
              label="Lancer au démarrage"
              checked={autostartEnabled}
              disabled={!loaded}
              onCheckedChange={(checked) => {
                void setAutostart(checked);
              }}
            />

            <Select
              id="inference-backend"
              label="Backend d'inférence"
              value={settings.inferenceBackend}
              disabled={!loaded || saving}
              options={[
                {
                  value: "auto",
                  label: "Automatique (GPU si disponible, sinon CPU)",
                },
                { value: "cpu", label: "CPU uniquement" },
              ]}
              onChange={(value) => {
                void setInferenceBackend(value);
              }}
            />

            {inferenceInfo && (
              <p className="text-caption text-ash">
                Compilé avec support {inferenceInfo.compiled_backend} — backend
                actif : {inferenceInfo.active_backend}.
              </p>
            )}
          </SettingsSection>

          {errorMessage && (
            <p className="text-body-sm text-accent-red">{errorMessage}</p>
          )}

          <footer className="flex flex-wrap items-center justify-between gap-4 border-t border-divider-soft pt-6">
            <Button
              variant="ghost"
              disabled={!loaded || saving}
              onClick={() => {
                void resetSettings();
              }}
            >
              Réinitialiser
            </Button>
            <Button variant="primary" disabled>
              {saving ? "Enregistrement…" : "Enregistrement automatique"}
            </Button>
          </footer>
        </div>
    </div>
  );
}
