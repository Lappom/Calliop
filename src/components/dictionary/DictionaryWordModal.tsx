import { useEffect, useState, type FormEvent } from "react";
import { Button } from "../ui/Button";
import { Modal } from "../ui/Modal";
import { TextInput } from "../ui/TextInput";

interface DictionaryWordModalProps {
  open: boolean;
  onClose: () => void;
  busy: boolean;
  errorMessage: string | null;
  mode: "create" | "edit";
  initialWord?: string;
  onSubmit: (word: string) => Promise<boolean>;
}

export function DictionaryWordModal({
  open,
  onClose,
  busy,
  errorMessage,
  mode,
  initialWord = "",
  onSubmit,
}: DictionaryWordModalProps) {
  const [word, setWord] = useState(initialWord);
  const isCreate = mode === "create";

  useEffect(() => {
    if (open) {
      setWord(initialWord);
    }
  }, [open, initialWord]);

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    void onSubmit(word).then((success) => {
      if (success) {
        onClose();
      }
    });
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={isCreate ? "Nouveau mot" : "Modifier le mot"}
      description={
        isCreate
          ? "Ajoutez un mot ou nom propre injecté dans le prompt Whisper pour améliorer la transcription."
          : "Modifiez l'orthographe ou la casse du mot enregistré."
      }
    >
      <form className="space-y-4" onSubmit={handleSubmit}>
        <TextInput
          label="Mot ou expression"
          value={word}
          onChange={(event) => setWord(event.target.value)}
          placeholder="Ex. Calliop, Kubernetes, Dupont-Martin…"
          disabled={busy}
        />

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
            disabled={busy || !word.trim()}
          >
            {busy
              ? "Enregistrement…"
              : isCreate
                ? "Ajouter"
                : "Enregistrer"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
