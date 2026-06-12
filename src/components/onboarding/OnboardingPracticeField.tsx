import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { PracticePhase } from "../../hooks/useOnboardingPractice";

export interface OnboardingPracticeFieldHandle {
  focus: () => void;
  readValue: () => string;
}

interface OnboardingPracticeFieldProps {
  phase: PracticePhase;
  syncValue: string;
  onValueChange: (value: string) => void;
  disabled?: boolean;
}

export const OnboardingPracticeField = forwardRef<
  OnboardingPracticeFieldHandle,
  OnboardingPracticeFieldProps
>(function OnboardingPracticeField(
  { phase, syncValue, onValueChange, disabled = false },
  ref,
) {
  const { t } = useTranslation();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const isRecording = phase === "recording";

  useEffect(() => {
    const el = textareaRef.current;
    if (!el || el.value === syncValue) {
      return;
    }
    el.value = syncValue;
  }, [syncValue]);

  useImperativeHandle(ref, () => ({
    focus: () => {
      textareaRef.current?.focus();
    },
    readValue: () => textareaRef.current?.value ?? "",
  }));

  return (
    <textarea
      ref={textareaRef}
      defaultValue=""
      disabled={disabled}
      rows={4}
      placeholder={t("onboarding.practice.placeholder")}
      aria-label={t("onboarding.practice.placeholder")}
      onInput={(event) => {
        onValueChange(event.currentTarget.value);
      }}
      className={[
        "w-full resize-y rounded-lg border bg-surface-card p-4",
        "font-[family-name:var(--font-ui)] text-body-sm text-ink",
        "placeholder:text-ash",
        "transition-[border-color,box-shadow] duration-200 ease-out",
        "focus:outline-none focus:border-ink",
        isRecording
          ? "border-accent-green shadow-[0_0_0_1px_var(--color-accent-green-glow)]"
          : "border-hairline-strong",
      ]
        .filter(Boolean)
        .join(" ")}
    />
  );
});
