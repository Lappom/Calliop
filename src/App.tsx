import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import "./App.css";

type PipelineState =
  | "idle"
  | "recording"
  | "transcribing"
  | "injecting"
  | "error";

interface PipelineStatePayload {
  state: PipelineState;
  message?: string | null;
}

interface ModelDownloadProgress {
  downloaded: number;
  total: number | null;
  percent: number;
  source: string;
}

const STATE_LABELS: Record<PipelineState, string> = {
  idle: "En attente",
  recording: "Écoute en cours…",
  transcribing: "Transcription…",
  injecting: "Injection du texte…",
  error: "Erreur",
};

function App() {
  const [pipelineState, setPipelineState] = useState<PipelineState>("idle");
  const [lastTranscript, setLastTranscript] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [modelReady, setModelReady] = useState(false);
  const [modelProgress, setModelProgress] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    const setup = async () => {
      try {
        const state = await invoke<string>("get_pipeline_state");
        if (!cancelled) {
          setPipelineState(state as PipelineState);
        }
      } catch {
        // Backend not ready yet.
      }

      try {
        await invoke("ensure_model");
        if (!cancelled) {
          setModelReady(true);
          setModelProgress(null);
        }
      } catch (err) {
        if (!cancelled) {
          setErrorMessage(String(err));
        }
      }
    };

    void setup();

    const unlisteners = Promise.all([
      listen<PipelineStatePayload>("pipeline-state", (event) => {
        setPipelineState(event.payload.state);
        if (event.payload.state === "error") {
          setErrorMessage(event.payload.message ?? "Erreur inconnue");
        } else {
          setErrorMessage(null);
        }
        if (event.payload.message && event.payload.state === "idle") {
          setLastTranscript(event.payload.message);
        }
      }),
      listen("model-ready", () => {
        setModelReady(true);
        setModelProgress(null);
      }),
      listen<ModelDownloadProgress>("model-download-progress", (event) => {
        setModelProgress(event.payload.percent);
      }),
    ]);

    return () => {
      cancelled = true;
      void unlisteners.then((drops) => drops.forEach((drop) => drop()));
    };
  }, []);

  return (
    <main className="app">
      <header className="app__header">
        <h1>Calliop</h1>
        <p className="app__tagline">Dictée vocale locale</p>
      </header>

      <section className="app__status" aria-live="polite">
        {!modelReady && modelProgress !== null && (
          <p className="app__download">
            Téléchargement du modèle Whisper : {modelProgress.toFixed(0)} %
          </p>
        )}
        {!modelReady && modelProgress === null && !errorMessage && (
          <p className="app__download">Préparation du modèle Whisper…</p>
        )}
        {modelReady && (
          <p className="app__state">
            État : <strong>{STATE_LABELS[pipelineState]}</strong>
          </p>
        )}
        {errorMessage && <p className="app__error">{errorMessage}</p>}
        {lastTranscript && (
          <p className="app__transcript">
            Dernière dictée : <em>{lastTranscript}</em>
          </p>
        )}
      </section>

      <section className="app__help">
        <p>
          Appuyez sur <kbd>Alt</kbd> + <kbd>Espace</kbd> pour démarrer / arrêter
          la dictée.
        </p>
        <p className="app__hint">
          Placez le curseur dans Notepad, Word ou un navigateur avant de dicter.
        </p>
      </section>
    </main>
  );
}

export default App;
