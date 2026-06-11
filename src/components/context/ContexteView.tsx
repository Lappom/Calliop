import { useState } from "react";
import {
  useAppContext,
  type AppContextMatchType,
  type ToneProfile,
} from "../../hooks/useAppContext";
import { BadgePill } from "../ui/BadgePill";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { TextInput } from "../ui/TextInput";

const MATCH_TYPE_LABELS: Record<AppContextMatchType, string> = {
  exe: "Nom de l'exécutable",
  title_contains: "Titre contient",
};

const TONE_LABELS: Record<ToneProfile, string> = {
  default: "Neutre",
  casual: "Décontracté",
  formal: "Formel",
  technical: "Technique",
};

export function ContexteView() {
  const [pattern, setPattern] = useState("");
  const [matchType, setMatchType] = useState<AppContextMatchType>("exe");
  const [tone, setTone] = useState<ToneProfile>("casual");
  const {
    rules,
    activeWindow,
    loaded,
    busy,
    errorMessage,
    addRule,
    removeRule,
    refreshActiveWindow,
  } = useAppContext();

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">Contexte</h1>
        <p className="text-body-sm text-charcoal">
          Adaptez le ton de la dictée selon l&apos;application active. Le profil
          de ton s&apos;applique uniquement lorsque l&apos;auto-édition IA est
          activée dans les réglages.
        </p>
      </header>

      <Card variant="bordered" className="space-y-4 p-6">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div>
            <p className="text-heading-sm mb-1 text-ink">Application active</p>
            {activeWindow ? (
              <div className="space-y-1">
                <p className="text-body-sm text-charcoal">
                  {activeWindow.title || "(sans titre)"}
                </p>
                <p className="text-caption text-ash">{activeWindow.exeName}</p>
              </div>
            ) : (
              <p className="text-body-sm text-ash">
                Aucune fenêtre détectée (ou fenêtre Calliop au premier plan).
              </p>
            )}
          </div>
          <Button
            type="button"
            variant="ghost"
            disabled={busy}
            onClick={() => void refreshActiveWindow()}
          >
            Actualiser
          </Button>
        </div>
      </Card>

      <Card variant="bordered" className="space-y-6 p-6">
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            void addRule(pattern, matchType, tone).then((inserted) => {
              if (inserted) {
                setPattern("");
              }
            });
          }}
        >
          <TextInput
            label="Motif"
            value={pattern}
            onChange={(event) => setPattern(event.target.value)}
            placeholder={
              matchType === "exe" ? 'Ex. "slack.exe"' : 'Ex. "Outlook"'
            }
            disabled={!loaded || busy}
          />
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="flex flex-col gap-2">
              <label htmlFor="match-type" className="text-body-sm text-charcoal">
                Type de correspondance
              </label>
              <select
                id="match-type"
                value={matchType}
                onChange={(event) =>
                  setMatchType(event.target.value as AppContextMatchType)
                }
                disabled={!loaded || busy}
                className="rounded-md border border-hairline-strong bg-surface-card px-3.5 py-2.5 text-body-md text-ink"
              >
                <option value="exe">{MATCH_TYPE_LABELS.exe}</option>
                <option value="title_contains">
                  {MATCH_TYPE_LABELS.title_contains}
                </option>
              </select>
            </div>
            <div className="flex flex-col gap-2">
              <label htmlFor="tone-profile" className="text-body-sm text-charcoal">
                Profil de ton
              </label>
              <select
                id="tone-profile"
                value={tone}
                onChange={(event) =>
                  setTone(event.target.value as ToneProfile)
                }
                disabled={!loaded || busy}
                className="rounded-md border border-hairline-strong bg-surface-card px-3.5 py-2.5 text-body-md text-ink"
              >
                {(Object.keys(TONE_LABELS) as ToneProfile[]).map((key) => (
                  <option key={key} value={key}>
                    {TONE_LABELS[key]}
                  </option>
                ))}
              </select>
            </div>
          </div>
          <Button
            type="submit"
            variant="primary"
            disabled={!loaded || busy || !pattern.trim()}
          >
            Ajouter une règle
          </Button>
        </form>

        {errorMessage && (
          <p className="text-body-sm text-accent-red">{errorMessage}</p>
        )}

        <div className="border-t border-divider-soft pt-4">
          {!loaded ? (
            <p className="text-body-sm text-ash">Chargement…</p>
          ) : rules.length === 0 ? (
            <p className="text-body-sm text-ash">Aucune règle configurée.</p>
          ) : (
            <ul className="space-y-3">
              {rules.map((rule) => (
                <li
                  key={rule.id}
                  className="flex flex-wrap items-center justify-between gap-3 rounded-md border border-hairline-soft px-4 py-3"
                >
                  <div className="min-w-0 flex-1">
                    <p className="text-body-md truncate text-ink">
                      {rule.pattern}
                    </p>
                    <p className="text-caption text-ash">
                      {MATCH_TYPE_LABELS[rule.matchType]}
                    </p>
                  </div>
                  <div className="flex items-center gap-3">
                    <BadgePill>{TONE_LABELS[rule.tone]}</BadgePill>
                    <Button
                      type="button"
                      variant="ghost"
                      disabled={busy}
                      onClick={() => void removeRule(rule.id)}
                    >
                      Supprimer
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </Card>
    </div>
  );
}
