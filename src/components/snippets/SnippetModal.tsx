import { ArrowRight } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import { Button } from "../ui/Button";
import { CodeWindow } from "../ui/CodeWindow";
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
  onPreview: (content: string) => Promise<string>;
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
  onPreview,
}: SnippetModalProps) {
  const [trigger, setTrigger] = useState(initialTrigger);
  const [content, setContent] = useState(initialContent);
  const [preview, setPreview] = useState("");
  useEffect(() => {
    if (open) {
      setTrigger(initialTrigger);
      setContent(initialContent);
      setPreview("");
    }
  }, [open, initialTrigger, initialContent]);

  useEffect(() => {
    if (!open) {
      return;
    }

    const trimmed = content.trim();
    if (!trimmed) {
      setPreview("");
      return;
    }

    const timer = window.setTimeout(() => {
      void onPreview(trimmed).then(setPreview);
    }, 300);

    return () => window.clearTimeout(timer);
  }, [open, content, onPreview]);

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
      size="lg"
      title={isCreate ? "Nouveau snippet" : "Modifier le snippet"}
      description={
        isCreate
          ? "Définissez un déclencheur vocal et le texte inséré après la transcription."
          : "Modifiez le déclencheur ou le texte inséré."
      }
    >
      <form className="space-y-4" onSubmit={handleSubmit}>
        <TextInput
          label="Déclencheur vocal"
          value={trigger}
          onChange={(event) => setTrigger(event.target.value)}
          placeholder='Ex. "ma signature"'
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
            placeholder={"Cordialement,\n{{nom}}"}
            disabled={busy}
            rows={4}
            className="rounded-md border border-hairline-strong bg-surface-card px-3.5 py-2.5 text-body-md text-ink focus:border-ink focus:outline-none disabled:opacity-50"
          />
        </div>

        {content.trim() && (
          <div className="flex flex-col gap-2">
            <span className="text-body-sm text-charcoal">Aperçu</span>
            {trigger.trim() && (
              <div className="flex min-w-0 items-center gap-2 text-body-sm">
                <span className="shrink-0 font-medium text-ink">{trigger.trim()}</span>
                <ArrowRight size={14} className="shrink-0 text-ash" aria-hidden />
                <span className="min-w-0 truncate text-charcoal">
                  {preview || "…"}
                </span>
              </div>
            )}
            <CodeWindow showTrafficLights={false} className="text-left">
              <span className="whitespace-pre-wrap break-words">
                {preview || "…"}
              </span>
            </CodeWindow>
          </div>
        )}

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
