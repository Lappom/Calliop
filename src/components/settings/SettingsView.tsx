import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { useSettings } from "../../hooks/useSettings";
import { BadgePill } from "../ui/BadgePill";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Kbd } from "../ui/Kbd";
import { ProgressBar } from "../ui/ProgressBar";
import { TextInput } from "../ui/TextInput";

type SettingsTab = "general" | "models" | "shortcuts" | "advanced";

const tabs: { id: SettingsTab; label: string }[] = [
  { id: "general", label: "Général" },
  { id: "models", label: "Modèles" },
  { id: "shortcuts", label: "Raccourcis" },
  { id: "advanced", label: "Avancé" },
];

function hotkeyParts(hotkey: string): string[] {
  return hotkey.split("+").map((part) => part.trim());
}

function formatHotkeyLabel(hotkey: string): string {
  return hotkey.replace(/Space/g, "Espace");
}

function captureHotkey(event: KeyboardEvent): string | null {
  event.preventDefault();
  event.stopPropagation();

  if (event.key === "Escape") {
    return null;
  }

  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Super");

  const key = event.key;
  if (["Control", "Alt", "Shift", "Meta"].includes(key)) {
    return null;
  }

  const normalizedKey =
    key === " " ? "Space" : key.length === 1 ? key.toUpperCase() : key;

  if (parts.length === 0) {
    return null;
  }

  parts.push(normalizedKey);
  return parts.join("+");
}

export function SettingsView() {
  const [activeTab, setActiveTab] = useState<SettingsTab>("general");
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
    formatBytes,
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
    deleteModel,
  } = useSettings();

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
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">Paramètres</h1>
        <p className="text-body-sm text-charcoal">
          Les réglages sont enregistrés localement sur cet appareil.
        </p>
      </header>

      <div className="flex flex-col gap-6 lg:flex-row lg:gap-8">
        <nav
          className={[
            "flex shrink-0 gap-2 overflow-x-auto pb-1 lg:w-[180px] lg:max-w-[180px] lg:flex-col lg:overflow-visible lg:pb-0",
          ].join(" ")}
          aria-label="Sections des réglages"
        >
          {tabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => setActiveTab(tab.id)}
              className={[
                "shrink-0 rounded-md px-3 py-2 text-left",
                "font-[family-name:var(--font-body)] text-sm font-medium tracking-wide",
                "transition-colors duration-150",
                "lg:w-full",
                activeTab === tab.id
                  ? "border border-hairline-strong bg-surface-elevated text-ink"
                  : "border border-transparent text-body hover:text-ink lg:hover:bg-surface-elevated/50",
              ].join(" ")}
              aria-current={activeTab === tab.id ? "true" : undefined}
            >
              {tab.label}
            </button>
          ))}
        </nav>

        <div className="min-w-0 flex-1">
      {activeTab === "general" && (
        <Card variant="bordered" className="space-y-6 p-6">
          <div className="flex flex-col gap-2">
            <label htmlFor="language" className="text-body-sm text-charcoal">
              Langue de dictée
            </label>
            <select
              id="language"
              value={settings.sttLanguage}
              disabled={!loaded || saving}
              onChange={(e) => {
                const value = e.target.value;
                if (value === "fr" || value === "en" || value === "auto") {
                  void setSttLanguage(value);
                }
              }}
              className="h-10 rounded-md border border-hairline-strong bg-surface-card px-3.5 text-ink disabled:opacity-50"
            >
              <option value="fr">Français</option>
              <option value="en">Anglais</option>
              <option value="auto">Détection automatique</option>
            </select>
          </div>

          <label className="flex cursor-pointer items-center justify-between gap-4">
            <span className="text-body-md text-ink">Auto-edits IA</span>
            <input
              type="checkbox"
              checked={settings.autoEdit}
              disabled={!loaded || saving}
              onChange={(e) => {
                void setAutoEdit(e.target.checked);
              }}
              className="size-4 rounded-sm border border-hairline-strong bg-surface-card accent-accent-blue disabled:opacity-50"
            />
          </label>

          <p className="text-caption text-ash">
            {settings.autoEdit
              ? "Nettoie fillers, ponctuation et reformulation légère via un modèle local."
              : "Mode verbatim : la transcription brute est injectée sans post-traitement LLM."}
          </p>

          {settings.autoEdit && llmProgress !== null && (
            <ProgressBar
              value={llmProgress}
              label={`Téléchargement LLM (${llmProgressModel ?? settings.llmModel})`}
            />
          )}

          <label className="flex cursor-pointer items-center justify-between gap-4">
            <span className="text-body-md text-ink">
              Apprentissage automatique des corrections
            </span>
            <input
              type="checkbox"
              checked={settings.autoLearn}
              disabled={!loaded || saving}
              onChange={(e) => {
                void setAutoLearn(e.target.checked);
              }}
              className="size-4 rounded-sm border border-hairline-strong bg-surface-card accent-accent-blue disabled:opacity-50"
            />
          </label>

          <p className="text-caption text-ash">
            {settings.autoLearn
              ? "Détecte les corrections dans l'application cible et enrichit le dictionnaire localement."
              : "Désactivé : seules les corrections manuelles sont prises en compte."}
          </p>

          {errorMessage && (
            <p className="text-body-sm text-accent-red">{errorMessage}</p>
          )}
        </Card>
      )}

      {activeTab === "models" && (
        <Card variant="bordered" className="space-y-6 p-6">
          <div className="flex flex-col gap-2">
            <label htmlFor="whisper-model" className="text-body-sm text-charcoal">
              Modèle Whisper (STT)
            </label>
            <select
              id="whisper-model"
              value={settings.whisperModel}
              disabled={!loaded || saving}
              onChange={(e) => {
                const value = e.target.value;
                if (value === "small" || value === "distil-fr-dec16") {
                  void setWhisperModel(value);
                }
              }}
              className="h-10 rounded-md border border-hairline-strong bg-surface-card px-3.5 text-ink disabled:opacity-50"
            >
              <option value="small">Rapide — Small (~466 Mo)</option>
              <option value="distil-fr-dec16">
                Équilibré — Distil FR dec16 (~755 Mo)
              </option>
            </select>
          </div>

          {sttProgress !== null && (
            <ProgressBar
              value={sttProgress}
              label={`Téléchargement Whisper (${sttProgressModel ?? settings.whisperModel})`}
            />
          )}

          <div className="flex flex-col gap-2">
            <label htmlFor="llm-model" className="text-body-sm text-charcoal">
              Modèle LLM (auto-edits)
            </label>
            <select
              id="llm-model"
              value={settings.llmModel}
              disabled={!loaded || saving}
              onChange={(e) => {
                const value = e.target.value;
                if (
                  value === "qwen3-0.6b" ||
                  value === "qwen3-1.7b" ||
                  value === "qwen3-4b"
                ) {
                  void setLlmModel(value);
                }
              }}
              className="h-10 rounded-md border border-hairline-strong bg-surface-card px-3.5 text-ink disabled:opacity-50"
            >
              <option value="qwen3-0.6b">
                Qwen3 0.6B — latence CPU réduite (~484 Mo)
              </option>
              <option value="qwen3-1.7b">
                Qwen3 1.7B — meilleure fidélité FR (~1,1 Go)
              </option>
              <option value="qwen3-4b">
                Qwen3 4B — haute fidélité (~2,5 Go, GPU recommandé)
              </option>
            </select>
          </div>

          {inferenceInfo && (
            <div className="flex flex-wrap items-center gap-2">
              <BadgePill active>
                Backend actif : {inferenceInfo.active_backend.toUpperCase()}
              </BadgePill>
              {inferenceInfo.gpu_available ? (
                <BadgePill>GPU Vulkan disponible</BadgePill>
              ) : (
                <BadgePill>CPU uniquement (build sans GPU)</BadgePill>
              )}
            </div>
          )}

          {modelsStatus && (
            <div className="space-y-3 border-t border-divider-soft pt-4">
              <p className="text-body-sm text-charcoal">Modèles installés</p>
              <ul className="space-y-2 text-body-sm text-body">
                {[...modelsStatus.whisper, ...modelsStatus.llm].map((entry) => (
                  <li
                    key={`${entry.id}-${entry.label}`}
                    className="flex flex-wrap items-center justify-between gap-2"
                  >
                    <span>
                      {entry.label}
                      {entry.active && (
                        <span className="ml-2 text-accent-blue">(actif)</span>
                      )}
                    </span>
                    <span className="flex items-center gap-2">
                      <span className="text-ash">
                        {entry.installed
                          ? formatBytes(entry.size_bytes)
                          : "non installé"}
                      </span>
                      {entry.installed && !entry.active && (
                        <Button
                          variant="ghost"
                          className="!px-2 !py-1 text-xs"
                          disabled={saving}
                          onClick={() => {
                            const kind = modelsStatus.whisper.some(
                              (w) => w.id === entry.id,
                            )
                              ? "whisper"
                              : "llm";
                            void deleteModel(kind, entry.id);
                          }}
                        >
                          Supprimer
                        </Button>
                      )}
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {errorMessage && (
            <p className="text-body-sm text-accent-red">{errorMessage}</p>
          )}
        </Card>
      )}

      {activeTab === "shortcuts" && (
        <Card variant="bordered" className="space-y-6 p-6">
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
          {(errorMessage || hotkeyRestoreError) && (
            <div className="space-y-2">
              {errorMessage && (
                <p className="text-body-sm text-accent-red">{errorMessage}</p>
              )}
              {hotkeyRestoreError && (
                <>
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
                </>
              )}
            </div>
          )}
        </Card>
      )}

      {activeTab === "advanced" && (
        <Card variant="bordered" className="space-y-6 p-6">
          <label className="flex cursor-pointer items-center justify-between gap-4">
            <span className="text-body-md text-ink">Mises à jour automatiques</span>
            <input
              type="checkbox"
              checked={settings.autoUpdate}
              disabled={!loaded || saving}
              onChange={(e) => {
                void setAutoUpdate(e.target.checked);
              }}
              className="size-4 rounded-sm border border-hairline-strong bg-surface-card accent-accent-blue disabled:opacity-50"
            />
          </label>

          <p className="text-caption text-ash">
            {settings.autoUpdate
              ? "Vérifie les nouvelles versions sur GitHub au démarrage et installe les mises à jour signées."
              : "Désactivé : aucune vérification réseau pour les mises à jour de l'application."}
          </p>

          <label className="flex cursor-pointer items-center justify-between gap-4">
            <span className="text-body-md text-ink">Lancer au démarrage</span>
            <input
              type="checkbox"
              checked={autostartEnabled}
              disabled={!loaded || saving}
              onChange={(e) => {
                void setAutostart(e.target.checked);
              }}
              className="size-4 rounded-sm border border-hairline-strong bg-surface-card accent-accent-blue disabled:opacity-50"
            />
          </label>

          <div className="flex flex-col gap-2">
            <label
              htmlFor="inference-backend"
              className="text-body-sm text-charcoal"
            >
              Backend d&apos;inférence
            </label>
            <select
              id="inference-backend"
              value={settings.inferenceBackend}
              disabled={!loaded || saving}
              onChange={(e) => {
                const value = e.target.value;
                if (value === "auto" || value === "cpu") {
                  void setInferenceBackend(value);
                }
              }}
              className="h-10 rounded-md border border-hairline-strong bg-surface-card px-3.5 text-ink disabled:opacity-50"
            >
              <option value="auto">
                Automatique (GPU si disponible, sinon CPU)
              </option>
              <option value="cpu">CPU uniquement</option>
            </select>
          </div>

          {inferenceInfo && (
            <p className="text-caption text-ash">
              Compilé avec support {inferenceInfo.compiled_backend} — backend
              actif : {inferenceInfo.active_backend}.
            </p>
          )}

          {errorMessage && (
            <p className="text-body-sm text-accent-red">{errorMessage}</p>
          )}
        </Card>
      )}

      <footer className="mt-6 flex flex-wrap items-center justify-between gap-4 border-t border-divider-soft pt-6">
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
    </div>
  );
}
