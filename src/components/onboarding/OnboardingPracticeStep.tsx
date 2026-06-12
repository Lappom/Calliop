import { useEffect, useRef } from "react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";
import type { PracticePhase } from "../../hooks/useOnboardingPractice";
import { pickVariants, successPopVariants } from "../../lib/motion/variants";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import { OnboardingHotkeyCoach } from "./OnboardingHotkeyCoach";
import {
  OnboardingPracticeField,
  type OnboardingPracticeFieldHandle,
} from "./OnboardingPracticeField";

interface OnboardingPracticeStepProps {
  hotkey: string;
  phase: PracticePhase;
  practiceText: string;
  pipelineState: string;
  partialTranscript: string;
  preparing: boolean;
  prepareError: string | null;
  pipelineError: string | null;
  onEnter: () => Promise<void>;
  onExit: () => Promise<void>;
  onValueChange: (value: string) => void;
}

export function OnboardingPracticeStep({
  hotkey,
  phase,
  practiceText,
  pipelineState,
  partialTranscript,
  preparing,
  prepareError,
  pipelineError,
  onEnter,
  onExit,
  onValueChange,
}: OnboardingPracticeStepProps) {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const fieldRef = useRef<OnboardingPracticeFieldHandle>(null);
  const enteredRef = useRef(false);

  useEffect(() => {
    if (enteredRef.current) {
      return;
    }
    enteredRef.current = true;
    void onEnter().then(() => {
      fieldRef.current?.focus();
    });
    return () => {
      enteredRef.current = false;
      void onExit();
    };
  }, [onEnter, onExit]);

  const showError = Boolean(prepareError || pipelineError);
  const successVariants = pickVariants(successPopVariants, reducedMotion);

  useEffect(() => {
    if (pipelineState !== "idle") {
      return;
    }
    const value = fieldRef.current?.readValue() ?? "";
    if (value.trim()) {
      onValueChange(value);
    }
  }, [pipelineState, onValueChange]);

  return (
    <>
      <h2 className="text-heading-md mb-2 text-ink">
        {t("onboarding.practice.title")}
      </h2>
      <p className="text-body-sm mb-2 text-charcoal">
        {t("onboarding.practice.instructions")}
      </p>
      <p className="text-caption mb-4 text-ash">
        {t("onboarding.practice.hintFocus")}
      </p>

      <OnboardingHotkeyCoach
        hotkey={hotkey}
        phase={phase}
        partialTranscript={partialTranscript}
        shake={showError}
      />

      <div className="mt-4 space-y-2">
        <p className="text-caption text-ash">{t("onboarding.practice.hintHotkey")}</p>
        <OnboardingPracticeField
          ref={fieldRef}
          phase={phase}
          syncValue={practiceText}
          onValueChange={onValueChange}
          disabled={preparing}
        />
      </div>

      {preparing && (
        <p className="text-body-sm mt-3 text-charcoal">
          {t("onboarding.practice.preparing")}
        </p>
      )}

      {prepareError && (
        <p className="text-body-sm mt-3 text-accent-red">{prepareError}</p>
      )}

      {pipelineError && (
        <p className="text-body-sm mt-3 text-accent-red">{pipelineError}</p>
      )}

      {phase === "success" && (
        <motion.p
          className="text-body-sm mt-3 text-accent-green"
          variants={successVariants}
          initial="initial"
          animate="animate"
        >
          {t("onboarding.practice.validated")}
        </motion.p>
      )}
    </>
  );
}
