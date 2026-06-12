import { Check, Loader2, Mic } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { PracticePhase } from "../../hooks/useOnboardingPractice";
import { useReducedMotion } from "../../lib/motion/useReducedMotion";
import { formatHotkeyDisplay, hotkeyParts } from "../main/mainUtils";
import { hotkeyPartLabel } from "../settings/settingsUtils";
import { Kbd } from "../ui/Kbd";

interface OnboardingHotkeyCoachProps {
  hotkey: string;
  phase: PracticePhase;
  partialTranscript: string;
  shake?: boolean;
}

const COACH_MESSAGE_KEYS: Record<PracticePhase, string> = {
  idle: "onboarding.coach.waiting",
  recording: "onboarding.coach.recording",
  processing: "onboarding.coach.processing",
  success: "onboarding.coach.success",
  error: "onboarding.coach.error",
};

function CoachIcon({ phase }: { phase: PracticePhase }) {
  if (phase === "success") {
    return <Check size={18} strokeWidth={2} className="text-accent-green" aria-hidden />;
  }
  if (phase === "processing") {
    return (
      <Loader2
        size={18}
        strokeWidth={2}
        className="animate-spin text-accent-blue"
        aria-hidden
      />
    );
  }
  return (
    <Mic
      size={18}
      strokeWidth={1.75}
      className={phase === "recording" ? "text-accent-green" : "text-charcoal"}
      aria-hidden
    />
  );
}

export function OnboardingHotkeyCoach({
  hotkey,
  phase,
  partialTranscript,
  shake = false,
}: OnboardingHotkeyCoachProps) {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const parts = hotkeyParts(hotkey);

  return (
    <div
      className={[
        "rounded-lg border border-hairline-strong bg-surface-elevated p-4",
        "transition-[border-color,background-color] duration-200 ease-out",
        phase === "recording" ? "border-accent-green/40" : "",
        shake && !reducedMotion ? "animate-onboarding-coach-shake" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className="mb-3 flex items-center gap-2">
        <CoachIcon phase={phase} />
        <p className="text-caption m-0 font-medium text-ink">
          {t(COACH_MESSAGE_KEYS[phase])}
        </p>
      </div>

      <div className="flex flex-wrap items-center gap-1.5">
        {parts.map((part, index) => (
          <span key={`${part}-${index}`} className="inline-flex items-center gap-1.5">
            {index > 0 && (
              <span className="text-caption text-ash">{t("common.plusSeparator")}</span>
            )}
            <Kbd>{hotkeyPartLabel(t, part)}</Kbd>
          </span>
        ))}
        <span className="text-caption text-ash">
          ({formatHotkeyDisplay(hotkey, t)})
        </span>
      </div>

      {phase === "recording" && partialTranscript && (
        <p className="text-body-sm mt-3 mb-0 text-charcoal">
          {partialTranscript}
        </p>
      )}
    </div>
  );
}
