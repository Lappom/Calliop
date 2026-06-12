import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useOnboarding } from "../../hooks/useOnboarding";
import { translateError } from "../../lib/translateError";
import { OnboardingStepTransition } from "../motion/OnboardingStepTransition";
import { MainHotkeyGuide } from "../main/MainHotkeyGuide";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { SectionGlow } from "../layout/SectionGlow";
import { StatusDot } from "../ui/StatusDot";
import { ProgressBar } from "../ui/ProgressBar";
import { OnboardingPracticeStep } from "./OnboardingPracticeStep";
import { OnboardingStepIndicator } from "./OnboardingStepIndicator";

interface OnboardingViewProps {
  onComplete: () => void;
}

export function OnboardingView({ onComplete }: OnboardingViewProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState(1);
  const [direction, setDirection] = useState(1);
  const stepRef = useRef(step);
  const {
    modelProgress,
    modelReady,
    modelLoading,
    modelError,
    audioLevel,
    micProbing,
    practicePhase,
    practiceText,
    practiceReady,
    partialTranscript,
    pipelineState,
    preparing,
    prepareError,
    pipelineError,
    hotkey,
    retryEnsureModel,
    startMicProbe,
    stopMicProbe,
    enterPracticeStep,
    exitPracticeStep,
    syncPracticeFromField,
    completeOnboarding,
  } = useOnboarding();

  const totalSteps = 4;
  const isFirst = step === 1;
  const isLast = step === totalSteps;

  const goToStep = (next: number) => {
    const clamped = Math.min(totalSteps, Math.max(1, next));
    setDirection(clamped > stepRef.current ? 1 : -1);
    stepRef.current = clamped;
    setStep(clamped);
  };

  useEffect(() => {
    stepRef.current = step;
  }, [step]);

  useEffect(() => {
    if (step === 2 || !micProbing) {
      return;
    }
    void stopMicProbe();
  }, [step, micProbing, stopMicProbe]);

  const goToNextStep = async () => {
    if (step === 2 && micProbing) {
      await stopMicProbe();
    }
    if (step === 3) {
      await exitPracticeStep();
    }
    goToStep(step + 1);
  };

  const goToPreviousStep = async () => {
    if (step === 3) {
      await exitPracticeStep();
    }
    goToStep(step - 1);
  };

  const handleFinish = async () => {
    await completeOnboarding();
    onComplete();
  };

  const continueDisabled =
    (step === 1 && (modelLoading || (!modelReady && !modelError))) ||
    (step === 3 && !practiceReady);

  return (
    <div className="calliop-scroll mx-auto flex min-h-0 flex-1 w-full max-w-[880px] flex-col gap-8 overflow-y-auto bg-canvas p-4 sm:p-8">
      <OnboardingStepIndicator currentStep={step} />

      <SectionGlow glow={isLast ? "green" : "blue"}>
        <Card variant="bordered" className="p-4 sm:p-6 lg:p-8">
          <OnboardingStepTransition stepKey={step} direction={direction}>
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
                    className="h-full bg-accent-blue transition-[width] duration-75"
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
              <OnboardingPracticeStep
                hotkey={hotkey}
                phase={practicePhase}
                practiceText={practiceText}
                pipelineState={pipelineState}
                partialTranscript={partialTranscript}
                preparing={preparing}
                prepareError={prepareError}
                pipelineError={pipelineError}
                onEnter={enterPracticeStep}
                onExit={exitPracticeStep}
                onValueChange={syncPracticeFromField}
              />
            )}

            {step === 4 && (
              <>
                <div className="mb-4 flex items-center gap-3">
                  <StatusDot color="green" />
                  <h2 className="text-heading-md m-0 text-ink">
                    {t("onboarding.done.title")}
                  </h2>
                </div>
                <p className="text-body-md mb-6 text-body">
                  {t("onboarding.done.subtitle")}
                </p>
                <MainHotkeyGuide />
              </>
            )}
          </OnboardingStepTransition>
        </Card>
      </SectionGlow>

      <div className="flex items-center justify-between gap-4">
        <Button
          variant="ghost"
          disabled={isFirst}
          onClick={() => void goToPreviousStep()}
        >
          {t("onboarding.navigation.previous")}
        </Button>
        {!isLast ? (
          <Button
            variant="primary"
            disabled={continueDisabled}
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
