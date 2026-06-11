import { useEffect, useState } from "react";
import { useOnboarding } from "../../hooks/useOnboarding";
import { BadgePill } from "../ui/BadgePill";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { CodeWindow } from "../ui/CodeWindow";
import { SectionGlow } from "../layout/SectionGlow";
import { StatusDot } from "../ui/StatusDot";
import { ProgressBar } from "../ui/ProgressBar";
import { Kbd } from "../ui/Kbd";

const STEPS = [
  { id: 1, label: "Bienvenue" },
  { id: 2, label: "Micro" },
  { id: 3, label: "Test" },
  { id: 4, label: "Terminé" },
];

interface OnboardingViewProps {
  onComplete: () => void;
}

function formatHotkey(hotkey: string): string {
  return hotkey.replace(/Space/g, "Espace");
}

export function OnboardingView({ onComplete }: OnboardingViewProps) {
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

  const isFirst = step === 1;
  const isLast = step === STEPS.length;
  const hotkeyParts = hotkey.split("+").map((p) => p.trim());

  useEffect(() => {
    if (step === 2 || !micProbing) return;
    void stopMicProbe();
  }, [step, micProbing, stopMicProbe]);

  const goToNextStep = async () => {
    if (step === 2 && micProbing) {
      await stopMicProbe();
    }
    setStep((s) => Math.min(STEPS.length, s + 1));
  };

  const handleFinish = async () => {
    await completeOnboarding();
    onComplete();
  };

  return (
    <div className="flex min-h-screen flex-col gap-8 bg-canvas p-8">
      <div className="flex flex-wrap gap-2">
        {STEPS.map((s) => (
          <BadgePill key={s.id} active={s.id === step}>
            {s.id}. {s.label}
          </BadgePill>
        ))}
      </div>

      <SectionGlow glow={isLast ? "green" : "blue"}>
        <Card variant="bordered" className="p-6 sm:p-8">
          {step === 1 && (
            <>
              <h1 className="text-display-serif mb-4 text-4xl text-ink">
                Bienvenue sur Calliop
              </h1>
              <p className="text-subtitle mb-4 text-charcoal">
                Dictez dans n&apos;importe quelle application, sans cloud.
              </p>
              {modelProgress !== null && !modelReady && (
                <ProgressBar
                  value={modelProgress}
                  label="Téléchargement du modèle Whisper…"
                />
              )}
              {modelLoading && modelProgress === null && !modelReady && (
                <p className="text-body-sm text-charcoal">
                  Chargement du modèle de transcription…
                </p>
              )}
              {modelReady && (
                <p className="text-body-sm text-accent-green">
                  Modèle de transcription prêt.
                </p>
              )}
              {modelError && (
                <div className="space-y-3">
                  <p className="text-body-sm text-accent-red">{modelError}</p>
                  <Button
                    variant="ghost"
                    disabled={modelLoading}
                    onClick={() => void retryEnsureModel()}
                  >
                    Réessayer le téléchargement
                  </Button>
                  <p className="text-caption text-ash">
                    Vous pouvez continuer pour tester le micro ; la dictée
                    nécessite le modèle.
                  </p>
                </div>
              )}
            </>
          )}

          {step === 2 && (
            <>
              <h2 className="text-heading-md mb-4 text-ink">
                Autoriser le microphone
              </h2>
              <p className="text-body-md mb-4 text-body">
                Calliop a besoin d&apos;accéder à votre micro. Cliquez sur
                « Tester le micro » — Windows affichera une demande de
                permission au premier enregistrement.
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
                {micProbing ? "Arrêter le test" : "Tester le micro"}
              </Button>
            </>
          )}

          {step === 3 && (
            <>
              <h2 className="text-heading-md mb-4 text-ink">
                Testez une dictée
              </h2>
              <p className="text-body-sm mb-4 text-charcoal">
                Cliquez sur « Démarrer la dictée », puis dites « Bonjour, ceci
                est un test ».
              </p>
              <CodeWindow showTrafficLights={false}>
                {dictationText || "En attente de votre voix…"}
              </CodeWindow>
              <div className="mt-4 flex items-center gap-3">
                <Button
                  variant="primary"
                  disabled={micProbing || micProbeStopping}
                  onClick={() => void runDictationTest()}
                >
                  {pipelineState === "recording"
                    ? "Arrêter la dictée"
                    : "Démarrer la dictée"}
                </Button>
                <span className="text-caption text-ash">
                  État : {pipelineState}
                </span>
              </div>
            </>
          )}

          {step === 4 && (
            <>
              <div className="mb-4 flex items-center gap-3">
                <StatusDot color="green" />
                <h2 className="text-heading-md m-0 text-ink">
                  Vous êtes prêt
                </h2>
              </div>
              <p className="text-body-md text-body">
                Utilisez{" "}
                {hotkeyParts.map((part, index) => (
                  <span key={`${part}-${index}`}>
                    {index > 0 && " + "}
                    <Kbd>{part === "Space" ? "Espace" : part}</Kbd>
                  </span>
                ))}{" "}
                ({formatHotkey(hotkey)}) partout où vous pouvez taper du texte.
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
          Précédent
        </Button>
        {!isLast ? (
          <Button
            variant="primary"
            disabled={step === 1 && (modelLoading || (!modelReady && !modelError))}
            onClick={() => void goToNextStep()}
          >
            Continuer
          </Button>
        ) : (
          <Button variant="primary" onClick={() => void handleFinish()}>
            Commencer
          </Button>
        )}
      </div>
    </div>
  );
}
