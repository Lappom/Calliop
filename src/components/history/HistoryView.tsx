import { useEffect, useRef, useState } from "react";
import { useHistory } from "../../hooks/useHistory";
import { SectionGlow } from "../layout/SectionGlow";
import { Card } from "../ui/Card";
import { TextInput } from "../ui/TextInput";

function formatDateTime(iso: string): string {
  const date = new Date(iso.includes("T") ? iso : `${iso.replace(" ", "T")}Z`);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  return new Intl.DateTimeFormat("fr-FR", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

const CLICK_DELAY_MS = 250;

export function HistoryView() {
  const [searchQuery, setSearchQuery] = useState("");
  const clickTimeoutRef = useRef<number | null>(null);
  const {
    entries,
    loaded,
    busy,
    errorMessage,
    entryFeedback,
    loadEntries,
    copyEntry,
    reinjectEntry,
  } = useHistory();

  useEffect(() => {
    const handle = window.setTimeout(() => {
      void loadEntries(searchQuery);
    }, 300);
    return () => window.clearTimeout(handle);
  }, [searchQuery, loadEntries]);

  useEffect(() => {
    return () => {
      if (clickTimeoutRef.current !== null) {
        window.clearTimeout(clickTimeoutRef.current);
      }
    };
  }, []);

  const handleEntryClick = (id: number) => {
    if (busy) {
      return;
    }
    if (clickTimeoutRef.current !== null) {
      window.clearTimeout(clickTimeoutRef.current);
    }
    clickTimeoutRef.current = window.setTimeout(() => {
      void copyEntry(id);
      clickTimeoutRef.current = null;
    }, CLICK_DELAY_MS);
  };

  const handleEntryDoubleClick = (id: number) => {
    if (busy) {
      return;
    }
    if (clickTimeoutRef.current !== null) {
      window.clearTimeout(clickTimeoutRef.current);
      clickTimeoutRef.current = null;
    }
    void reinjectEntry(id);
  };

  return (
    <div className="flex flex-col gap-8">
      <header>
        <h1 className="text-heading-md mb-2 text-ink">Historique</h1>
        <p className="text-body-sm text-charcoal">
          Vos dictées passées. Clic pour copier · double-clic pour réinjecter
          dans l&apos;application active.
        </p>
      </header>

      <SectionGlow glow="blue">
        <TextInput
          label="Rechercher"
          value={searchQuery}
          onChange={(event) => setSearchQuery(event.target.value)}
          placeholder="Mots-clés, application, extrait de texte…"
          disabled={!loaded || busy}
        />
      </SectionGlow>

      {errorMessage && (
        <p className="text-body-sm text-accent-red">{errorMessage}</p>
      )}

      {!loaded && (
        <p className="text-body-sm text-charcoal">Chargement…</p>
      )}

      {loaded && entries.length === 0 && (
        <Card variant="bordered" className="p-6">
          <p className="text-body-sm m-0 text-charcoal">
            {searchQuery.trim()
              ? "Aucun résultat pour cette recherche."
              : "Aucune dictée enregistrée. Dictez avec Alt+Espace pour remplir l'historique."}
          </p>
        </Card>
      )}

      {entries.length > 0 && (
        <ul className="m-0 flex list-none flex-col gap-3 p-0">
          {entries.map((entry) => {
            const feedback = entryFeedback[entry.id];
            return (
              <li key={entry.id}>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => handleEntryClick(entry.id)}
                  onDoubleClick={() => handleEntryDoubleClick(entry.id)}
                  className={[
                    "w-full rounded-lg border border-hairline-strong bg-surface-card p-4 sm:p-5",
                    "text-left transition-colors duration-150",
                    "hover:border-hairline-strong hover:bg-surface-elevated",
                    "focus-visible:border-ink focus-visible:outline-none",
                    "disabled:cursor-not-allowed disabled:opacity-50",
                  ].join(" ")}
                >
                  <p className="text-body-md m-0 whitespace-pre-wrap text-ink">
                    {entry.text}
                  </p>
                  <p className="text-caption mt-3 text-ash">
                    {formatDateTime(entry.created_at)}
                    {entry.appExe ? ` · ${entry.appExe}` : ""}
                    {` · ${entry.wordCount} mot${entry.wordCount > 1 ? "s" : ""}`}
                    {feedback === "copied" && (
                      <span className="text-accent-green"> · Copié</span>
                    )}
                    {feedback === "injected" && (
                      <span className="text-accent-green"> · Réinjecté</span>
                    )}
                  </p>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
