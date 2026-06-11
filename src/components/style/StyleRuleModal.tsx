import { useEffect, useMemo, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import type {
  AppContextMatchType,
  ToneProfile,
} from "../../hooks/useAppContext";
import { Button } from "../ui/Button";
import { Modal } from "../ui/Modal";
import { TextInput } from "../ui/TextInput";
import { getMatchTypeLabels } from "./styleUtils";
import { ToneProfilePicker } from "./ToneProfilePicker";

interface StyleRuleModalProps {
  open: boolean;
  onClose: () => void;
  busy: boolean;
  errorMessage: string | null;
  initialPattern?: string;
  initialMatchType?: AppContextMatchType;
  initialTone?: ToneProfile;
  onSubmit: (
    pattern: string,
    matchType: AppContextMatchType,
    tone: ToneProfile,
  ) => Promise<boolean>;
}

export function StyleRuleModal({
  open,
  onClose,
  busy,
  errorMessage,
  initialPattern = "",
  initialMatchType = "exe",
  initialTone = "casual",
  onSubmit,
}: StyleRuleModalProps) {
  const { t } = useTranslation();
  const matchTypeLabels = useMemo(() => getMatchTypeLabels(t), [t]);
  const [pattern, setPattern] = useState(initialPattern);
  const [matchType, setMatchType] =
    useState<AppContextMatchType>(initialMatchType);
  const [tone, setTone] = useState(initialTone);

  useEffect(() => {
    if (open) {
      setPattern(initialPattern);
      setMatchType(initialMatchType);
      setTone(initialTone);
    }
  }, [open, initialPattern, initialMatchType, initialTone]);

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    void onSubmit(pattern, matchType, tone).then((success) => {
      if (success) {
        onClose();
      }
    });
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("style.modal.title")}
      description={t("style.modal.description")}
      size="md"
    >
      <form className="space-y-5" onSubmit={handleSubmit}>
        <TextInput
          label={t("style.modal.patternLabel")}
          value={pattern}
          onChange={(event) => setPattern(event.target.value)}
          placeholder={
            matchType === "exe"
              ? t("style.modal.patternExePlaceholder")
              : t("style.modal.patternTitlePlaceholder")
          }
          disabled={busy}
        />

        <div className="flex flex-col gap-2">
          <span className="text-body-sm text-charcoal">
            {t("style.modal.matchTypeLabel")}
          </span>
          <div className="flex flex-wrap gap-2">
            {(Object.keys(matchTypeLabels) as AppContextMatchType[]).map(
              (type) => (
                <button
                  key={type}
                  type="button"
                  disabled={busy}
                  onClick={() => setMatchType(type)}
                  className={[
                    "rounded-md border px-3 py-2",
                    "font-[family-name:var(--font-ui)] text-sm font-medium",
                    "transition-colors duration-150 disabled:opacity-40",
                    matchType === type
                      ? "border-hairline-strong bg-surface-elevated text-ink"
                      : "border-hairline bg-surface-card text-charcoal hover:text-ink",
                  ].join(" ")}
                  aria-pressed={matchType === type}
                >
                  {matchTypeLabels[type]}
                </button>
              ),
            )}
          </div>
        </div>

        <ToneProfilePicker value={tone} disabled={busy} onChange={setTone} />

        {errorMessage && (
          <p className="text-body-sm text-accent-red">{errorMessage}</p>
        )}

        <div className="flex justify-end gap-2 pt-2">
          <Button type="button" variant="ghost" disabled={busy} onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button type="submit" variant="primary" disabled={busy}>
            {t("common.save")}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
