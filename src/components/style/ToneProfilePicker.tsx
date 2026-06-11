import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ToneProfile } from "../../hooks/useAppContext";
import { getToneMeta, TONE_PROFILES } from "./styleUtils";

interface ToneProfilePickerProps {
  value: ToneProfile;
  disabled?: boolean;
  onChange: (tone: ToneProfile) => void;
}

export function ToneProfilePicker({
  value,
  disabled,
  onChange,
}: ToneProfilePickerProps) {
  const { t } = useTranslation();
  const toneMeta = useMemo(() => getToneMeta(t), [t]);

  return (
    <fieldset className="m-0 border-0 p-0">
      <legend className="text-body-sm mb-3 text-charcoal">
        {t("style.modal.toneLegend")}
      </legend>
      <div className="grid gap-2 sm:grid-cols-2">
        {TONE_PROFILES.map((tone) => {
          const meta = toneMeta[tone];
          const selected = value === tone;
          return (
            <button
              key={tone}
              type="button"
              disabled={disabled}
              onClick={() => onChange(tone)}
              className={[
                "rounded-lg border px-4 py-3 text-left transition-colors duration-150",
                "disabled:cursor-not-allowed disabled:opacity-40",
                selected
                  ? "border-hairline-strong bg-surface-elevated"
                  : "border-hairline bg-surface-card hover:border-hairline-strong hover:bg-surface-elevated/60",
              ].join(" ")}
              aria-pressed={selected}
            >
              <span
                className="mb-1.5 inline-block size-1.5 rounded-full"
                style={{ backgroundColor: meta.accent }}
                aria-hidden
              />
              <span className="block text-body-sm font-medium text-ink">
                {meta.label}
              </span>
              <span className="mt-1 block text-caption leading-snug text-ash">
                {meta.description}
              </span>
            </button>
          );
        })}
      </div>
    </fieldset>
  );
}
