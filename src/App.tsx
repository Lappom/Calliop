import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { AppFrame } from "./components/layout/AppFrame";
import { AppShell } from "./components/layout/AppShell";
import { LlmSkipToast } from "./components/layout/LlmSkipToast";
import { ModelDownloadToasts } from "./components/layout/ModelDownloadToasts";
import { UpdateToast } from "./components/layout/UpdateToast";
import { PageTransition } from "./components/motion/PageTransition";
import { StyleView } from "./components/style/StyleView";
import { DictionaryView } from "./components/dictionary/DictionaryView";
import { HistoryView } from "./components/history/HistoryView";
import { InsightView } from "./components/insight/InsightView";
import { AchievementsView } from "./components/achievements/AchievementsView";
import { AchievementUnlockQueue } from "./components/achievements/AchievementUnlockQueue";
import { MainView } from "./components/main/MainView";
import { OnboardingView } from "./components/onboarding/OnboardingView";
import { SettingsModal } from "./components/settings/SettingsModal";
import type { SettingsSectionId } from "./components/settings/settingsUtils";
import { SnippetsView } from "./components/snippets/SnippetsView";
import { usePipelineState } from "./hooks/usePipelineState";
import type { AppView } from "./lib/views";
import { isAppView, isSettingsNavigation } from "./lib/views";

function App() {
  const { t } = useTranslation();
  const [currentView, setCurrentView] = useState<AppView>("main");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsSection, setSettingsSection] =
    useState<SettingsSectionId>("general");
  const [onboardingChecked, setOnboardingChecked] = useState(false);
  const [showOnboarding, setShowOnboarding] = useState(false);
  const pipeline = usePipelineState();

  const openSettings = () => {
    setSettingsSection("general");
    setSettingsOpen(true);
  };

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
      if (isSettingsNavigation(view)) {
        setSettingsSection("general");
        setSettingsOpen(true);
        return;
      }
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
        <UpdateToast />
        <AchievementUnlockQueue />
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
        <UpdateToast />
        <AchievementUnlockQueue />
      </>
    );
  }

  return (
    <>
      <AppFrame>
        <AppShell
          currentView={currentView}
          onNavigate={setCurrentView}
          settingsOpen={settingsOpen}
          onOpenSettings={openSettings}
        >
          <PageTransition viewKey={currentView}>
            {currentView === "main" && <MainView {...pipeline} />}
            {currentView === "dictionary" && <DictionaryView />}
            {currentView === "snippets" && <SnippetsView />}
            {currentView === "style" && <StyleView />}
            {currentView === "history" && <HistoryView />}
            {currentView === "insight" && (
              <InsightView latencyMetrics={pipeline.latencyMetrics} />
            )}
            {currentView === "achievements" && <AchievementsView />}
          </PageTransition>
        </AppShell>
      </AppFrame>
      <SettingsModal
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        activeSection={settingsSection}
        onSectionChange={setSettingsSection}
      />
      <ModelDownloadToasts />
      <LlmSkipToast />
      <UpdateToast />
      <AchievementUnlockQueue />
    </>
  );
}

export default App;
