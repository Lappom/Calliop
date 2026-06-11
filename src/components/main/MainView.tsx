import { Card } from "../ui/Card";
import { Button } from "../ui/Button";
import { Kbd } from "../ui/Kbd";
import { ProgressBar } from "../ui/ProgressBar";
import { SectionGlow } from "../layout/SectionGlow";
import { StatusDot } from "../ui/StatusDot";
import { useTranscriptCorrection } from "../../hooks/useTranscriptCorrection";
import {
  pipelineGlow,
  pipelineStatusColor,
  STATE_LABELS,
  type LatencyMetricsPayload,
  type PipelineState,
} from "../../hooks/usePipelineState";

interface MainViewProps {
  pipelineState: PipelineState;
  lastTranscript: string | null;
  transcriptRevision: number;
  partialTranscript: string;
  errorMessage: string | null;
  modelReady: boolean;
  modelProgress: number | null;
  latencyMetrics: LatencyMetricsPayload | null;
}

export function MainView({
  pipelineState,
  lastTranscript,
  transcriptRevision,
  partialTranscript,
  errorMessage,
  modelReady,
  modelProgress,
  latencyMetrics,
}: MainViewProps) {
  const glow = pipelineGlow(pipelineState, Boolean(errorMessage));
  const statusColor = pipelineStatusColor(
    pipelineState,
    Boolean(errorMessage),
  );
  const {
    editedTranscript,
    setEditedTranscript,
    applyCorrection,
    learning,
    learnedWords,
    errorMessage: correctionError,
    hasChanges,
  } = useTranscriptCorrection(lastTranscript, transcriptRevision);

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
            {partialTranscript && pipelineState === "recording" && (
              <p className="text-body-sm mt-4 text-charcoal">
                {partialTranscript}
              </p>
            )}
            {errorMessage && (
              <p className="text-body-sm mt-4 text-accent-red">{errorMessage}</p>
            )}
            {lastTranscript && (
              <div className="mt-6 space-y-3">
                <p className="text-caption text-charcoal">
                  Dernière dictée — modifiez pour corriger et enrichir le
                  dictionnaire
                </p>
                <textarea
                  value={editedTranscript}
                  onChange={(event) => setEditedTranscript(event.target.value)}
                  onBlur={() => {
                    if (hasChanges) {
                      void applyCorrection();
                    }
                  }}
                  rows={4}
                  className={[
                    "w-full rounded-md border border-hairline-strong",
                    "bg-surface-card px-3.5 py-2.5 text-ink",
                    "font-[family-name:var(--font-ui)] text-sm leading-relaxed",
                    "focus:border-ink focus:outline-none",
                  ].join(" ")}
                />
                <div className="flex flex-wrap items-center gap-3">
                  <Button
                    type="button"
                    variant="outline"
                    disabled={!hasChanges || learning}
                    onClick={() => {
                      void applyCorrection();
                    }}
                  >
                    {learning ? "Apprentissage…" : "Appliquer la correction"}
                  </Button>
                  {learnedWords.length > 0 && (
                    <p className="text-body-sm text-charcoal">
                      Ajouté au dictionnaire : {learnedWords.join(", ")}
                    </p>
                  )}
                </div>
                {correctionError && (
                  <p className="text-body-sm text-accent-red">{correctionError}</p>
                )}
              </div>
            )}
            {latencyMetrics && (
              <div className="mt-6 rounded-md border border-hairline bg-surface-muted/40 px-3 py-2">
                <p className="text-caption mb-1 text-charcoal">
                  Latence (debug)
                </p>
                <p className="text-body-sm m-0 font-mono text-ink">
                  STT {latencyMetrics.sttMs} ms
                  {latencyMetrics.llmMs > 0
                    ? ` · LLM ${latencyMetrics.llmMs} ms`
                    : ""}{" "}
                  · injection {latencyMetrics.injectMs} ms · total{" "}
                  {latencyMetrics.totalMs} ms
                </p>
              </div>
            )}
          </div>
        </Card>
      </SectionGlow>

      <section className="space-y-3">
        <p className="text-body-sm text-body">
          <Kbd>Alt</Kbd> + <Kbd>Espace</Kbd> : appui pour démarrer, réappui pour
          arrêter (toggle) ; maintien puis relâcher pour le push-to-talk.
        </p>
        <p className="text-caption text-ash">
          Placez le curseur dans Notepad, Word ou un navigateur avant de dicter.
          L&apos;application reste accessible via l&apos;icône dans la barre des
          tâches.
        </p>
      </section>
    </>
  );
}
