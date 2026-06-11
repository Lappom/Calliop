import { useState } from "react";
import { useSnippets } from "../../hooks/useSnippets";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { TextInput } from "../ui/TextInput";

export function SnippetsView() {
  const [newTrigger, setNewTrigger] = useState("");
  const [newContent, setNewContent] = useState("");
  const {
    snippets,
    loaded,
    busy,
    errorMessage,
    fileInputRef,
    addSnippet,
    removeSnippet,
    exportSnippets,
    openImportDialog,
    handleImportFile,
  } = useSnippets();

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">Snippets</h1>
        <p className="text-body-sm text-charcoal">
          Définissez des déclencheurs vocaux qui insèrent un texte complet après
          la transcription.
        </p>
      </header>

      <Card variant="bordered" className="space-y-6 p-6">
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            void addSnippet(newTrigger, newContent).then((inserted) => {
              if (inserted) {
                setNewTrigger("");
                setNewContent("");
              }
            });
          }}
        >
          <TextInput
            label="Déclencheur vocal"
            value={newTrigger}
            onChange={(event) => setNewTrigger(event.target.value)}
            placeholder='Ex. "mon calendrier"'
            disabled={!loaded || busy}
          />
          <div className="flex flex-col gap-2">
            <label htmlFor="snippet-content" className="text-body-sm text-charcoal">
              Texte à insérer
            </label>
            <textarea
              id="snippet-content"
              value={newContent}
              onChange={(event) => setNewContent(event.target.value)}
              placeholder="Ex. Voici mon lien Calendly : calendly.com/…"
              disabled={!loaded || busy}
              rows={4}
              className="rounded-md border border-hairline-strong bg-surface-card px-3.5 py-2.5 text-body-md text-ink"
            />
          </div>
          <Button
            type="submit"
            variant="primary"
            disabled={!loaded || busy || !newTrigger.trim() || !newContent.trim()}
          >
            Ajouter
          </Button>
        </form>

        <div className="flex flex-wrap gap-3 border-t border-divider-soft pt-4">
          <input
            ref={fileInputRef}
            type="file"
            accept=".json,application/json"
            className="hidden"
            onChange={(event) => {
              void handleImportFile(event);
            }}
          />
          <Button
            type="button"
            variant="ghost"
            disabled={!loaded || busy}
            onClick={openImportDialog}
          >
            Importer JSON
          </Button>
          <Button
            type="button"
            variant="ghost"
            disabled={!loaded || busy || snippets.length === 0}
            onClick={() => {
              void exportSnippets();
            }}
          >
            Exporter JSON
          </Button>
        </div>

        {!loaded && (
          <p className="text-body-sm text-charcoal">Chargement…</p>
        )}

        {loaded && snippets.length === 0 && (
          <p className="text-body-sm text-charcoal">
            Aucun snippet enregistré. Ajoutez un déclencheur ou importez un
            fichier JSON.
          </p>
        )}

        {snippets.length > 0 && (
          <ul className="divide-y divide-divider-soft rounded-md border border-hairline">
            {snippets.map((entry) => (
              <li
                key={entry.id}
                className="flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-start sm:justify-between"
              >
                <div className="min-w-0 space-y-1">
                  <p className="text-body-md text-ink">
                    <span className="text-accent-orange">&quot;{entry.trigger}&quot;</span>
                    {" → "}
                    <span className="text-charcoal">{entry.content}</span>
                  </p>
                </div>
                <Button
                  type="button"
                  variant="ghost"
                  disabled={busy}
                  onClick={() => {
                    void removeSnippet(entry.id);
                  }}
                >
                  Supprimer
                </Button>
              </li>
            ))}
          </ul>
        )}

        {errorMessage && (
          <p className="text-body-sm text-accent-red">{errorMessage}</p>
        )}
      </Card>
    </div>
  );
}
