import { useEffect, useId, useState, type FormEvent } from "react";
import { ArrowRight } from "lucide-react";
import { useUiLocale } from "../../i18n/useUiLocale";
import { Button } from "../ui/Button";
import { InfoTooltip } from "../ui/InfoTooltip";
import { Modal } from "../ui/Modal";
import { TextInput } from "../ui/TextInput";
import { Toggle } from "../ui/Toggle";

const inputClassName = [
  "h-10 min-w-0 flex-1 rounded-md border border-hairline-strong",
  "bg-surface-card px-3.5 py-2.5 text-ink",
  "font-[family-name:var(--font-ui)] text-sm leading-[1.43]",
  "placeholder:text-mute",
  "focus:border-ink focus:outline-none",
  "disabled:cursor-not-allowed disabled:opacity-40",
].join(" ");

interface DictionaryWordModalProps {
  open: boolean;
  onClose: () => void;
  busy: boolean;
  errorMessage: string | null;
  mode: "create" | "edit";
  initialWord?: string;
  initialMisspelling?: string | null;
  onSubmit: (word: string, misspelling?: string) => Promise<boolean>;
}

export function DictionaryWordModal({
  open,
  onClose,
  busy,
  errorMessage,
  mode,
  initialWord = "",
  initialMisspelling = null,
  onSubmit,
}: DictionaryWordModalProps) {
  const { t } = useUiLocale();
  const [word, setWord] = useState(initialWord);
  const [correctMisspelling, setCorrectMisspelling] = useState(false);
  const [misspelling, setMisspelling] = useState("");
  const toggleId = useId();
  const isCreate = mode === "create";
  const showCorrectionInputs =
    (isCreate && correctMisspelling) ||
    (!isCreate && Boolean(initialMisspelling));

  useEffect(() => {
    if (open) {
      setWord(initialWord);
      setCorrectMisspelling(false);
      setMisspelling(initialMisspelling ?? "");
    }
  }, [open, initialWord, initialMisspelling]);

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    void onSubmit(
      word,
      showCorrectionInputs && misspelling.trim() ? misspelling : undefined,
    ).then((success) => {
      if (success) {
        onClose();
      }
    });
  };

  const canSubmit =
    word.trim().length > 0 &&
    (!showCorrectionInputs || misspelling.trim().length > 0);

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={
        isCreate ? t("dictionary.modal.createTitle") : t("dictionary.modal.editTitle")
      }
      description={
        isCreate
          ? t("dictionary.modal.createDescription")
          : t("dictionary.modal.editDescription")
      }
    >
      <form className="space-y-4" onSubmit={handleSubmit}>
        {isCreate && (
          <div className="flex items-center justify-between gap-4">
            <div className="flex min-w-0 items-center gap-2">
              <label
                htmlFor={toggleId}
                className="text-body-md cursor-pointer text-ink"
              >
                {t("dictionary.modal.correctMisspelling")}
              </label>
              <InfoTooltip content={t("dictionary.modal.correctMisspellingHelp")} />
            </div>
            <Toggle
              id={toggleId}
              checked={correctMisspelling}
              disabled={busy}
              onCheckedChange={(checked) => {
                setCorrectMisspelling(checked);
                if (!checked) {
                  setMisspelling("");
                }
              }}
            />
          </div>
        )}

        {showCorrectionInputs ? (
          <div className="flex flex-col gap-2">
            <span className="text-body-sm text-charcoal">
              {t("dictionary.modal.wordLabel")}
            </span>
            <div className="flex min-w-0 items-center gap-2.5 sm:gap-3">
              <input
                type="text"
                value={misspelling}
                onChange={(event) => setMisspelling(event.target.value)}
                placeholder={t("dictionary.modal.incorrectPlaceholder")}
                disabled={busy || !isCreate}
                aria-label={t("dictionary.modal.incorrectAria")}
                className={inputClassName}
              />
              <ArrowRight
                size={16}
                strokeWidth={1.75}
                className="shrink-0 text-ash"
                aria-hidden
              />
              <input
                type="text"
                value={word}
                onChange={(event) => setWord(event.target.value)}
                placeholder={t("dictionary.modal.correctPlaceholder")}
                disabled={busy}
                aria-label={t("dictionary.modal.correctAria")}
                className={inputClassName}
              />
            </div>
            {!isCreate && initialMisspelling && (
              <p className="text-caption m-0 text-ash">
                {t("dictionary.modal.cannotEditMisspelling")}
              </p>
            )}
          </div>
        ) : (
          <TextInput
            label={t("dictionary.modal.wordOrExpression")}
            value={word}
            onChange={(event) => setWord(event.target.value)}
            placeholder={t("dictionary.modal.wordPlaceholder")}
            disabled={busy}
          />
        )}

        {errorMessage && (
          <p className="text-body-sm text-accent-red">{errorMessage}</p>
        )}

        <div className="flex flex-wrap justify-end gap-3 pt-2">
          <Button
            type="button"
            variant="outline"
            disabled={busy}
            onClick={onClose}
          >
            {t("common.cancel")}
          </Button>
          <Button
            type="submit"
            variant="primary"
            disabled={busy || !canSubmit}
          >
            {busy
              ? t("common.saving")
              : isCreate
                ? t("common.add")
                : t("common.save")}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
