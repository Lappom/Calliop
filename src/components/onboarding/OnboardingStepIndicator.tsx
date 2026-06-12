import { useTranslation } from "react-i18next";
import { BadgePill } from "../ui/BadgePill";

const STEP_KEYS = ["welcome", "microphone", "practice", "done"] as const;

interface OnboardingStepIndicatorProps {
  currentStep: number;
}

export function OnboardingStepIndicator({
  currentStep,
}: OnboardingStepIndicatorProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-wrap gap-2">
      {STEP_KEYS.map((key, index) => {
        const id = index + 1;
        const done = id < currentStep;
        const active = id === currentStep;

        return (
          <BadgePill
            key={key}
            active={active}
            className={[
              "transition-opacity duration-[var(--motion-base)]",
              done ? "text-charcoal opacity-70" : "",
            ]
              .filter(Boolean)
              .join(" ")}
          >
            {id}. {t(`onboarding.steps.${key}`)}
          </BadgePill>
        );
      })}
    </div>
  );
}
