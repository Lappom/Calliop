import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { AppFrame } from "./components/layout/AppFrame";
import { AppShell } from "./components/layout/AppShell";
import { LlmSkipToast } from "./components/layout/LlmSkipToast";
import { ModelDownloadToasts } from "./components/layout/ModelDownloadToasts";
import { PageTransition } from "./components/motion/PageTransition";
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
import { isAppView } from "./lib/views";

function App() {
  const { t } = useTranslation();
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

  useEffect(() => {
    const unlisten = listen<{ view: string }>("navigate-view", (event) => {
      const view = event.payload.view;
      if (isAppView(view)) {
        setCurrentView(view);
      }
    });
    return () => {
      void unlisten.then((drop) => drop());
    };
  }, []);

  if (!onboardingChecked) {
    return (
      <>
        <AppFrame>
          <div className="flex flex-1 items-center justify-center text-body">
            {t("common.loading")}
          </div>
        </AppFrame>
        <ModelDownloadToasts />
        <LlmSkipToast />
      </>
    );
  }

  if (showOnboarding) {
    return (
      <>
        <AppFrame>
          <OnboardingView
            onComplete={() => {
              setShowOnboarding(false);
              setCurrentView("main");
            }}
          />
        </AppFrame>
        <ModelDownloadToasts />
        <LlmSkipToast />
      </>
    );
  }

  return (
    <>
      <AppFrame>
        <AppShell currentView={currentView} onNavigate={setCurrentView}>
          <PageTransition viewKey={currentView}>
            {currentView === "main" && <MainView {...pipeline} />}
            {currentView === "dictionary" && <DictionaryView />}
            {currentView === "snippets" && <SnippetsView />}
            {currentView === "style" && <StyleView />}
            {currentView === "history" && <HistoryView />}
            {currentView === "insight" && (
              <InsightView latencyMetrics={pipeline.latencyMetrics} />
            )}
            {currentView === "settings" && <SettingsView />}
          </PageTransition>
        </AppShell>
      </AppFrame>
      <ModelDownloadToasts />
      <LlmSkipToast />
    </>
  );
}

export default App;
