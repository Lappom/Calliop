import { useEffect, useState, type FormEvent } from "react";
import { Button } from "../ui/Button";
import { Modal } from "../ui/Modal";
import { TextInput } from "../ui/TextInput";

interface SnippetModalProps {
  open: boolean;
  onClose: () => void;
  busy: boolean;
  errorMessage: string | null;
  mode: "create" | "edit";
  initialTrigger?: string;
  initialContent?: string;
  onSubmit: (trigger: string, content: string) => Promise<boolean>;
}

export function SnippetModal({
  open,
  onClose,
  busy,
  errorMessage,
  mode,
  initialTrigger = "",
  initialContent = "",
  onSubmit,
}: SnippetModalProps) {
  const [trigger, setTrigger] = useState(initialTrigger);
  const [content, setContent] = useState(initialContent);

  useEffect(() => {
    if (open) {
      setTrigger(initialTrigger);
      setContent(initialContent);
    }
  }, [open, initialTrigger, initialContent]);

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    void onSubmit(trigger, content).then((success) => {
      if (success) {
        onClose();
      }
    });
  };

  const isCreate = mode === "create";

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={isCreate ? "Nouveau snippet" : "Modifier le snippet"}
      description={
        isCreate
          ? "Définissez un déclencheur vocal et le texte qui sera inséré après la transcription."
          : "Modifiez le déclencheur ou le texte inséré."
      }
    >
      <form className="space-y-4" onSubmit={handleSubmit}>
        <TextInput
          label="Déclencheur vocal"
          value={trigger}
          onChange={(event) => setTrigger(event.target.value)}
          placeholder='Ex. "mon calendrier"'
          disabled={busy}
        />
        <div className="flex flex-col gap-2">
          <label htmlFor="snippet-content" className="text-body-sm text-charcoal">
            Texte à insérer
          </label>
          <textarea
            id="snippet-content"
            value={content}
            onChange={(event) => setContent(event.target.value)}
            placeholder="Ex. Voici mon lien Calendly : calendly.com/…"
            disabled={busy}
            rows={4}
            className="rounded-md border border-hairline-strong bg-surface-card px-3.5 py-2.5 text-body-md text-ink focus:border-ink focus:outline-none disabled:opacity-50"
          />
        </div>

        {errorMessage && (
          <p className="text-body-sm text-accent-red">{errorMessage}</p>
        )}

        <div className="flex flex-wrap justify-end gap-3 pt-2">
          <Button type="button" variant="outline" disabled={busy} onClick={onClose}>
            Annuler
          </Button>
          <Button
            type="submit"
            variant="primary"
            disabled={busy || !trigger.trim() || !content.trim()}
          >
            {busy ? "Enregistrement…" : isCreate ? "Ajouter" : "Enregistrer"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
