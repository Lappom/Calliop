import { useState } from "react";
import { BadgePill } from "../ui/BadgePill";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { CodeWindow } from "../ui/CodeWindow";
import { SectionGlow } from "../layout/SectionGlow";
import { StatusDot } from "../ui/StatusDot";

const STEPS = [
  { id: 1, label: "Bienvenue" },
  { id: 2, label: "Micro" },
  { id: 3, label: "Test" },
  { id: 4, label: "Terminé" },
];

export function OnboardingView() {
  const [step, setStep] = useState(1);

  const isFirst = step === 1;
  const isLast = step === STEPS.length;

  return (
    <div className="flex flex-col gap-8">
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
              <p className="text-subtitle text-charcoal">
                Dictez dans n&apos;importe quelle application, sans cloud.
              </p>
            </>
          )}

          {step === 2 && (
            <>
              <h2 className="text-heading-md mb-4 text-ink">
                Autoriser le microphone
              </h2>
              <p className="text-body-md text-body">
                Calliop a besoin d&apos;accéder à votre micro pour capturer la
                voix. Windows affichera une demande de permission au premier
                enregistrement.
              </p>
            </>
          )}

          {step === 3 && (
            <>
              <h2 className="text-heading-md mb-4 text-ink">
                Testez une dictée
              </h2>
              <p className="text-body-sm mb-4 text-charcoal">
                Dites « Bonjour, ceci est un test » dans le champ ci-dessous.
              </p>
              <CodeWindow showTrafficLights={false}>
                Bonjour, ceci est un test
              </CodeWindow>
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
                Utilisez Alt + Espace partout où vous pouvez taper du texte.
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
            onClick={() => setStep((s) => Math.min(STEPS.length, s + 1))}
          >
            Continuer
          </Button>
        ) : (
          <Button variant="primary" onClick={() => setStep(1)}>
            Recommencer
          </Button>
        )}
      </div>
    </div>
  );
}
