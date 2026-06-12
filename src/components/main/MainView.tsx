import { useTranslation } from "react-i18next";
import { Stagger } from "../motion/Stagger";
import type {
  LatencyMetricsPayload,
  PipelineState,
} from "../../hooks/usePipelineState";
import { useTranscriptCorrection } from "../../hooks/useTranscriptCorrection";
import { LatencySummary } from "./LatencySummary";
import { MainHotkeyGuide } from "./MainHotkeyGuide";
import { PipelineStatusCard } from "./PipelineStatusCard";
import { TranscriptCorrectionPanel } from "./TranscriptCorrectionPanel";

interface MainViewProps {
  pipelineState: PipelineState;
  lastTranscript: string | null;
  transcriptRevision: number;
  partialTranscript: string;
  errorMessage: string | null;
  modelReady: boolean;
  modelProgress: number | null;
  audioLevel: number;
  latencyMetrics: LatencyMetricsPayload | null;
  busyHint: string | null;
  sttSegmentsCompleted: number;
}

export function MainView({
  pipelineState,
  lastTranscript,
  transcriptRevision,
  partialTranscript,
  errorMessage,
  modelReady,
  modelProgress,
  audioLevel,
  latencyMetrics,
  busyHint,
  sttSegmentsCompleted,
}: MainViewProps) {
  const { t } = useTranslation();
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
    <Stagger className="flex flex-col gap-8" itemMotion="fade">
      <header>
        <h1 className="text-display-serif mb-2 text-4xl text-ink sm:text-5xl">
          {t("main.title")}
        </h1>
        <p className="text-subtitle text-charcoal">{t("main.subtitle")}</p>
      </header>

      <PipelineStatusCard
        pipelineState={pipelineState}
        errorMessage={errorMessage}
        modelReady={modelReady}
        modelProgress={modelProgress}
        partialTranscript={partialTranscript}
        audioLevel={audioLevel}
        busyHint={busyHint}
        sttSegmentsCompleted={sttSegmentsCompleted}
      />

      {modelReady && (
        <TranscriptCorrectionPanel
          editedTranscript={editedTranscript}
          hasTranscript={lastTranscript !== null}
          onChange={setEditedTranscript}
          onApply={() => {
            void applyCorrection();
          }}
          onBlurApply={() => {
            if (hasChanges) {
              void applyCorrection();
            }
          }}
          learning={learning}
          hasChanges={hasChanges}
          learnedWords={learnedWords}
          errorMessage={correctionError}
        />
      )}

      {latencyMetrics && modelReady && (
        <section className="flex flex-col gap-3">
          <p className="text-caption m-0 text-charcoal">
            {t("main.latency.title")}
          </p>
          <LatencySummary metrics={latencyMetrics} />
        </section>
      )}

      <section className="flex flex-col gap-3">
        <p className="text-caption m-0 text-charcoal">
          {t("keys.hotkeyGuideTitle")}
        </p>
        <MainHotkeyGuide />
      </section>
    </Stagger>
  );
}
