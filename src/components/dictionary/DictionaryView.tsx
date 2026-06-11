import { useState } from "react";
import { useDictionary } from "../../hooks/useDictionary";
import { BadgePill } from "../ui/BadgePill";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { TextInput } from "../ui/TextInput";

export function DictionaryView() {
  const [newWord, setNewWord] = useState("");
  const {
    words,
    loaded,
    busy,
    errorMessage,
    addWord,
    removeWord,
  } = useDictionary();

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">Dictionnaire</h1>
        <p className="text-body-sm text-charcoal">
          Mots et noms propres injectés dans le prompt Whisper pour améliorer
          la transcription.
        </p>
      </header>

      <Card variant="bordered" className="space-y-6 p-6">
        <form
          className="flex flex-wrap items-end gap-3"
          onSubmit={(event) => {
            event.preventDefault();
            void addWord(newWord).then((inserted) => {
              if (inserted) {
                setNewWord("");
              }
            });
          }}
        >
          <TextInput
            label="Ajouter un mot"
            value={newWord}
            onChange={(event) => setNewWord(event.target.value)}
            placeholder="Ex. Calliop, Kubernetes…"
            disabled={!loaded || busy}
            className="min-w-[220px] flex-1"
          />
          <Button
            type="submit"
            variant="primary"
            disabled={!loaded || busy || !newWord.trim()}
          >
            Ajouter
          </Button>
        </form>

        {!loaded && (
          <p className="text-body-sm text-charcoal">Chargement…</p>
        )}

        {loaded && words.length === 0 && (
          <p className="text-body-sm text-charcoal">
            Aucun mot enregistré. Ajoutez des noms propres ou corrigez une
            dictée depuis l&apos;accueil.
          </p>
        )}

        {words.length > 0 && (
          <ul className="divide-y divide-divider-soft rounded-md border border-hairline">
            {words.map((entry) => (
              <li
                key={entry.id}
                className="flex items-center justify-between gap-4 px-4 py-3"
              >
                <div className="flex min-w-0 items-center gap-3">
                  <span className="truncate text-body-md text-ink">
                    {entry.word}
                  </span>
                  <BadgePill>
                    {entry.source === "manual" ? "Manuel" : "Appris"}
                  </BadgePill>
                </div>
                <Button
                  type="button"
                  variant="ghost"
                  disabled={busy}
                  onClick={() => {
                    void removeWord(entry.id);
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
