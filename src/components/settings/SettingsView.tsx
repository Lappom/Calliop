import { useState } from "react";
import { BadgePill } from "../ui/BadgePill";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { Kbd } from "../ui/Kbd";
import { TextInput } from "../ui/TextInput";

type SettingsTab = "general" | "models" | "shortcuts" | "advanced";

const tabs: { id: SettingsTab; label: string }[] = [
  { id: "general", label: "Général" },
  { id: "models", label: "Modèles" },
  { id: "shortcuts", label: "Raccourcis" },
  { id: "advanced", label: "Avancé" },
];

export function SettingsView() {
  const [activeTab, setActiveTab] = useState<SettingsTab>("general");
  const [autoEdit, setAutoEdit] = useState(false);

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">Réglages</h1>
        <p className="text-body-sm text-charcoal">
          Configurez Calliop selon vos préférences (aperçu — non persisté).
        </p>
      </header>

      <nav
        className="flex flex-wrap gap-2"
        aria-label="Sections des réglages"
      >
        {tabs.map((tab) => (
          <button
            key={tab.id}
            type="button"
            onClick={() => setActiveTab(tab.id)}
            className={[
              "rounded-full px-3.5 py-1.5",
              "font-[family-name:var(--font-body)] text-sm font-medium tracking-wide",
              "transition-colors duration-150",
              activeTab === tab.id
                ? "bg-surface-elevated text-ink border border-hairline-strong"
                : "bg-surface-elevated text-body hover:text-ink",
            ].join(" ")}
            aria-current={activeTab === tab.id ? "true" : undefined}
          >
            {tab.label}
          </button>
        ))}
      </nav>

      {activeTab === "general" && (
        <Card variant="bordered" className="space-y-6 p-6">
          <div className="flex flex-col gap-2">
            <label htmlFor="language" className="text-body-sm text-charcoal">
              Langue de dictée
            </label>
            <select
              id="language"
              defaultValue="fr"
              className="h-10 rounded-md border border-hairline-strong bg-surface-card px-3.5 text-ink"
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
              checked={autoEdit}
              onChange={(e) => setAutoEdit(e.target.checked)}
              className="size-4 rounded-sm border border-hairline-strong bg-surface-card accent-accent-blue"
            />
          </label>
          <p className="text-caption text-ash">
            Nettoie automatiquement fillers et ponctuation (Phase 3).
          </p>
        </Card>
      )}

      {activeTab === "models" && (
        <Card variant="bordered" className="space-y-6 p-6">
          <div className="flex flex-col gap-2">
            <label htmlFor="whisper-model" className="text-body-sm text-charcoal">
              Modèle Whisper
            </label>
            <select
              id="whisper-model"
              defaultValue="small"
              className="h-10 rounded-md border border-hairline-strong bg-surface-card px-3.5 text-ink"
            >
              <option value="small">Small (~466 Mo)</option>
              <option value="medium">Medium (~1,5 Go)</option>
              <option value="large">Large (~3 Go)</option>
            </select>
          </div>
          <BadgePill>CPU — GPU bientôt disponible</BadgePill>
        </Card>
      )}

      {activeTab === "shortcuts" && (
        <Card variant="bordered" className="space-y-6 p-6">
          <TextInput
            label="Raccourci global"
            defaultValue="Alt+Espace"
            readOnly
          />
          <p className="text-body-sm text-charcoal">
            Aperçu : <Kbd>Alt</Kbd> + <Kbd>Espace</Kbd>
          </p>
        </Card>
      )}

      {activeTab === "advanced" && (
        <Card variant="bordered" className="space-y-4 p-6">
          <p className="text-body-md text-ink">Options avancées</p>
          <p className="text-caption text-ash">
            Latence debug, démarrage automatique, verbatim mode — Phase 4.
          </p>
        </Card>
      )}

      <footer className="flex flex-wrap items-center justify-between gap-4 border-t border-divider-soft pt-6">
        <Button variant="ghost">Réinitialiser</Button>
        <Button variant="primary" disabled>
          Enregistrer
        </Button>
      </footer>
    </div>
  );
}
