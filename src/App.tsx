import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AppShell } from "./components/layout/AppShell";
import { ModelDownloadToasts } from "./components/layout/ModelDownloadToasts";
import { StyleView } from "./components/style/StyleView";
import { DictionaryView } from "./components/dictionary/DictionaryView";
import { HistoryView } from "./components/history/HistoryView";
import { InsightView } from "./components/insight/InsightView";
import { MainView } from "./components/main/MainView";
import { OnboardingView } from "./components/onboarding/OnboardingView";
import { SettingsView } from "./components/settings/SettingsView";
import { SnippetsView } from "./components/snippets/SnippetsView";
import { usePipelineState } from "./hooks/usePipelineState";
import type { AppView } from "./lib/views";

function App() {
  const [currentView, setCurrentView] = useState<AppView>("main");
  const [onboardingChecked, setOnboardingChecked] = useState(false);
  const [showOnboarding, setShowOnboarding] = useState(false);
  const pipeline = usePipelineState();

  useEffect(() => {
    let cancelled = false;
    void invoke<boolean>("is_onboarding_done")
      .then((done) => {
        if (!cancelled) {
          setShowOnboarding(!done);
          setOnboardingChecked(true);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setOnboardingChecked(true);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!onboardingChecked) {
    return (
      <>
        <div className="flex min-h-screen items-center justify-center bg-canvas text-body">
          Chargement…
        </div>
        <ModelDownloadToasts />
      </>
    );
  }

  if (showOnboarding) {
    return (
      <>
        <OnboardingView
          onComplete={() => {
            setShowOnboarding(false);
            setCurrentView("main");
          }}
        />
        <ModelDownloadToasts />
      </>
    );
  }

  return (
    <>
      <AppShell currentView={currentView} onNavigate={setCurrentView}>
        {currentView === "main" && <MainView {...pipeline} />}
        {currentView === "dictionary" && <DictionaryView />}
        {currentView === "snippets" && <SnippetsView />}
        {currentView === "style" && <StyleView />}
        {currentView === "history" && <HistoryView />}
        {currentView === "insight" && (
          <InsightView latencyMetrics={pipeline.latencyMetrics} />
        )}
        {currentView === "settings" && <SettingsView />}
      </AppShell>
      <ModelDownloadToasts />
    </>
  );
}

export default App;
