import { Card } from "../ui/Card";
import { CodeWindow } from "../ui/CodeWindow";
import { Kbd } from "../ui/Kbd";
import { ProgressBar } from "../ui/ProgressBar";
import { SectionGlow } from "../layout/SectionGlow";
import { StatusDot } from "../ui/StatusDot";
import {
  pipelineGlow,
  pipelineStatusColor,
  STATE_LABELS,
  type PipelineState,
} from "../../hooks/usePipelineState";

interface MainViewProps {
  pipelineState: PipelineState;
  lastTranscript: string | null;
  errorMessage: string | null;
  modelReady: boolean;
  modelProgress: number | null;
}

export function MainView({
  pipelineState,
  lastTranscript,
  errorMessage,
  modelReady,
  modelProgress,
}: MainViewProps) {
  const glow = pipelineGlow(pipelineState, Boolean(errorMessage));
  const statusColor = pipelineStatusColor(
    pipelineState,
    Boolean(errorMessage),
  );

  return (
    <>
      <header className="mb-8">
        <h1 className="text-display-serif mb-2 text-4xl text-ink sm:text-5xl">
          Dictée vocale locale
        </h1>
        <p className="text-subtitle text-charcoal">
          Parlez dans n&apos;importe quelle application — 100&nbsp;% hors ligne.
        </p>
      </header>

      <SectionGlow glow={glow} className="mb-8">
        <Card variant="bordered" className="relative p-6 sm:p-8">
          <div aria-live="polite">
            {!modelReady && modelProgress !== null && (
              <ProgressBar
                value={modelProgress}
                label="Téléchargement du modèle Whisper"
              />
            )}
            {!modelReady && modelProgress === null && !errorMessage && (
              <p className="text-body-sm text-charcoal">
                Préparation du modèle Whisper…
              </p>
            )}
            {modelReady && (
              <div className="flex items-center gap-3">
                <StatusDot color={statusColor} />
                <p className="text-heading-sm m-0 text-ink">
                  {STATE_LABELS[pipelineState]}
                </p>
              </div>
            )}
            {errorMessage && (
              <p className="text-body-sm mt-4 text-accent-red">{errorMessage}</p>
            )}
            {lastTranscript && (
              <div className="mt-6">
                <p className="text-caption mb-2 text-charcoal">
                  Dernière dictée
                </p>
                <CodeWindow showTrafficLights={false}>
                  {lastTranscript}
                </CodeWindow>
              </div>
            )}
          </div>
        </Card>
      </SectionGlow>

      <section className="space-y-3">
        <p className="text-body-sm text-body">
          Appuyez sur <Kbd>Alt</Kbd> + <Kbd>Espace</Kbd> pour démarrer ou
          arrêter la dictée.
        </p>
        <p className="text-caption text-ash">
          Placez le curseur dans Notepad, Word ou un navigateur avant de dicter.
        </p>
      </section>
    </>
  );
}
