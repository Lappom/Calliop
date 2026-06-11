import { BookOpenCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { translateError } from "../../lib/translateError";
import { glowSurfaceClasses } from "../layout/glowSurface";
import { Button } from "../ui/Button";

interface TranscriptCorrectionPanelProps {
  editedTranscript: string;
  hasTranscript: boolean;
  onChange: (value: string) => void;
  onApply: () => void;
  onBlurApply: () => void;
  learning: boolean;
  hasChanges: boolean;
  learnedWords: string[];
  errorMessage: string | null;
}

export function TranscriptCorrectionPanel({
  editedTranscript,
  hasTranscript,
  onChange,
  onApply,
  onBlurApply,
  learning,
  hasChanges,
  learnedWords,
  errorMessage,
}: TranscriptCorrectionPanelProps) {
  const { t } = useTranslation();

  return (
    <section
      className={[
        glowSurfaceClasses("blue"),
        "rounded-lg border border-hairline-strong bg-surface-card p-5 sm:p-6",
      ].join(" ")}
      aria-labelledby="transcript-correction-heading"
    >
      <p
        id="transcript-correction-heading"
        className="text-caption relative m-0 text-charcoal"
      >
        {t("main.correction.heading")}
      </p>

      <div className="relative mt-4">
        <label htmlFor="transcript-correction" className="sr-only">
          {t("main.correction.srLabel")}
        </label>
        <textarea
          id="transcript-correction"
          value={editedTranscript}
          onChange={(event) => onChange(event.target.value)}
          onBlur={onBlurApply}
          rows={4}
          placeholder={
            hasTranscript ? undefined : t("main.correction.placeholder")
          }
          className={[
            "w-full rounded-md border border-hairline-strong",
            "bg-surface-deep px-3.5 py-2.5 text-ink",
            "font-[family-name:var(--font-ui)] text-sm leading-relaxed",
            "placeholder:text-stone focus:border-ink focus:outline-none",
            "disabled:cursor-not-allowed disabled:opacity-50",
          ].join(" ")}
          disabled={learning || !hasTranscript}
        />
      </div>

      <div className="relative mt-3 flex flex-wrap items-center gap-3">
        <Button
          type="button"
          variant="outline"
          disabled={!hasTranscript || !hasChanges || learning}
          onClick={onApply}
        >
          {learning
            ? t("main.correction.learning")
            : t("main.correction.apply")}
        </Button>
        {learnedWords.length > 0 && (
          <p className="text-body-sm m-0 flex flex-wrap items-center gap-2 text-charcoal">
            <span className="inline-flex items-center gap-1.5">
              <BookOpenCheck size={14} strokeWidth={1.75} aria-hidden />
              {t("main.correction.addedToDictionary")}
            </span>
            {learnedWords.map((word) => (
              <span
                key={word}
                className={[
                  "inline-flex items-center rounded-full border px-2.5 py-0.5",
                  "font-[family-name:var(--font-ui)] text-xs font-medium text-accent-green",
                  "border-[rgba(17,255,153,0.35)] bg-[rgba(34,255,153,0.12)]",
                ].join(" ")}
              >
                {word}
              </span>
            ))}
          </p>
        )}
      </div>

      {errorMessage && (
        <p className="text-body-sm relative mt-3 text-accent-red">
          {translateError(errorMessage, t)}
        </p>
      )}
    </section>
  );
}
