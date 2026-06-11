import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useOnboarding } from "../../hooks/useOnboarding";
import type { PipelineState } from "../../hooks/usePipelineState";
import { translateError } from "../../lib/translateError";
import { BadgePill } from "../ui/BadgePill";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { CodeWindow } from "../ui/CodeWindow";
import { SectionGlow } from "../layout/SectionGlow";
import { StatusDot } from "../ui/StatusDot";
import { ProgressBar } from "../ui/ProgressBar";
import { Kbd } from "../ui/Kbd";
import {
  formatHotkeyDisplay,
  hotkeyParts,
  pipelineStateLabel,
} from "../main/mainUtils";
import { hotkeyPartLabel } from "../settings/settingsUtils";

const HOTKEY_PARTS_MARKER = "%PARTS%";

function OnboardingHotkeyUsage({ hotkey }: { hotkey: string }) {
  const { t } = useTranslation();
  const parts = hotkeyParts(hotkey);
  const template = t("keys.onboardingUsage", {
    hotkeyParts: HOTKEY_PARTS_MARKER,
    hotkeyLabel: formatHotkeyDisplay(hotkey, t),
  });
  const segments = template.split(HOTKEY_PARTS_MARKER);

  return (
    <>
      {segments[0]}
      {parts.map((part, index) => (
        <span key={`${part}-${index}`}>
          {index > 0 && ` ${t("common.plusSeparator")} `}
          <Kbd>{hotkeyPartLabel(t, part)}</Kbd>
        </span>
      ))}
      {segments[1]}
    </>
  );
}

const STEP_IDS = [1, 2, 3, 4] as const;
const STEP_KEYS = ["welcome", "microphone", "test", "done"] as const;

interface OnboardingViewProps {
  onComplete: () => void;
}

export function OnboardingView({ onComplete }: OnboardingViewProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState(1);
  const {
    modelProgress,
    modelReady,
    modelLoading,
    modelError,
    audioLevel,
    micProbing,
    micProbeStopping,
    dictationText,
    pipelineState,
    hotkey,
    retryEnsureModel,
    startMicProbe,
    stopMicProbe,
    runDictationTest,
    completeOnboarding,
  } = useOnboarding();

  const steps = useMemo(
    () =>
      STEP_IDS.map((id, index) => ({
        id,
        label: t(`onboarding.steps.${STEP_KEYS[index]}`),
      })),
    [t],
  );

  const isFirst = step === 1;
  const isLast = step === steps.length;

  useEffect(() => {
    if (step === 2 || !micProbing) return;
    void stopMicProbe();
  }, [step, micProbing, stopMicProbe]);

  const goToNextStep = async () => {
    if (step === 2 && micProbing) {
      await stopMicProbe();
    }
    setStep((s) => Math.min(steps.length, s + 1));
  };

  const handleFinish = async () => {
    await completeOnboarding();
    onComplete();
  };

  const pipelineStateText = pipelineStateLabel(
    t,
    pipelineState as PipelineState,
  );

  return (
    <div className="calliop-scroll mx-auto flex min-h-0 flex-1 w-full max-w-[880px] flex-col gap-8 overflow-y-auto bg-canvas p-4 sm:p-8">
      <div className="flex flex-wrap gap-2">
        {steps.map((s) => (
          <BadgePill key={s.id} active={s.id === step}>
            {s.id}. {s.label}
          </BadgePill>
        ))}
      </div>

      <SectionGlow glow={isLast ? "green" : "blue"}>
        <Card variant="bordered" className="p-4 sm:p-6 lg:p-8">
          {step === 1 && (
            <>
              <h1 className="text-display-serif mb-4 text-4xl text-ink">
                {t("onboarding.welcome.title")}
              </h1>
              <p className="text-subtitle mb-4 text-charcoal">
                {t("onboarding.welcome.subtitle")}
              </p>
              {modelProgress !== null && !modelReady && (
                <ProgressBar
                  value={modelProgress}
                  label={t("onboarding.welcome.downloadWhisper")}
                />
              )}
              {modelLoading && modelProgress === null && !modelReady && (
                <p className="text-body-sm text-charcoal">
                  {t("onboarding.welcome.loadingModel")}
                </p>
              )}
              {modelReady && (
                <p className="text-body-sm text-accent-green">
                  {t("onboarding.welcome.modelReady")}
                </p>
              )}
              {modelError && (
                <div className="space-y-3">
                  <p className="text-body-sm text-accent-red">
                    {translateError(modelError, t)}
                  </p>
                  <Button
                    variant="ghost"
                    disabled={modelLoading}
                    onClick={() => void retryEnsureModel()}
                  >
                    {t("common.retryDownload")}
                  </Button>
                  <p className="text-caption text-ash">
                    {t("onboarding.welcome.continueWithoutModel")}
                  </p>
                </div>
              )}
            </>
          )}

          {step === 2 && (
            <>
              <h2 className="text-heading-md mb-4 text-ink">
                {t("onboarding.microphone.title")}
              </h2>
              <p className="text-body-md mb-4 text-body">
                {t("onboarding.microphone.description")}
              </p>
              <div className="mb-4 h-2 overflow-hidden rounded-full bg-surface-elevated">
                <div
                  className="h-full bg-accent-blue transition-all duration-75"
                  style={{
                    width: `${Math.min(100, Math.round(audioLevel * 100))}%`,
                  }}
                />
              </div>
              <Button
                variant="primary"
                onClick={() => {
                  if (micProbing) {
                    void stopMicProbe();
                  } else {
                    void startMicProbe();
                  }
                }}
              >
                {micProbing
                  ? t("onboarding.microphone.testStop")
                  : t("onboarding.microphone.testStart")}
              </Button>
            </>
          )}

          {step === 3 && (
            <>
              <h2 className="text-heading-md mb-4 text-ink">
                {t("onboarding.test.title")}
              </h2>
              <p className="text-body-sm mb-4 text-charcoal">
                {t("onboarding.test.instructions")}
              </p>
              <CodeWindow showTrafficLights={false}>
                {dictationText || t("onboarding.test.waiting")}
              </CodeWindow>
              <div className="mt-4 flex items-center gap-3">
                <Button
                  variant="primary"
                  disabled={micProbing || micProbeStopping}
                  onClick={() => void runDictationTest()}
                >
                  {pipelineState === "recording"
                    ? t("onboarding.test.stop")
                    : t("onboarding.test.start")}
                </Button>
                <span className="text-caption text-ash">
                  {t("onboarding.test.stateLabel")} {pipelineStateText}
                </span>
              </div>
            </>
          )}

          {step === 4 && (
            <>
              <div className="mb-4 flex items-center gap-3">
                <StatusDot color="green" />
                <h2 className="text-heading-md m-0 text-ink">
                  {t("onboarding.done.title")}
                </h2>
              </div>
              <p className="text-body-md text-body">
                <OnboardingHotkeyUsage hotkey={hotkey} />
              </p>
            </>
          )}
        </Card>
      </SectionGlow>

      <div className="flex items-center justify-between gap-4">
        <Button
          variant="ghost"
          disabled={isFirst}
          onClick={() => setStep((s) => Math.max(1, s - 1))}
        >
          {t("onboarding.navigation.previous")}
        </Button>
        {!isLast ? (
          <Button
            variant="primary"
            disabled={step === 1 && (modelLoading || (!modelReady && !modelError))}
            onClick={() => void goToNextStep()}
          >
            {t("onboarding.navigation.continue")}
          </Button>
        ) : (
          <Button variant="primary" onClick={() => void handleFinish()}>
            {t("onboarding.navigation.finish")}
          </Button>
        )}
      </div>
    </div>
  );
}
