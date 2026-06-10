import { useState } from "react";
import { AppShell } from "./components/layout/AppShell";
import { MainView } from "./components/main/MainView";
import { OnboardingView } from "./components/onboarding/OnboardingView";
import { SettingsView } from "./components/settings/SettingsView";
import { usePipelineState } from "./hooks/usePipelineState";
import type { AppView } from "./lib/views";

function App() {
  const [currentView, setCurrentView] = useState<AppView>("main");
  const pipeline = usePipelineState();

  return (
    <AppShell currentView={currentView} onNavigate={setCurrentView}>
      {currentView === "main" && <MainView {...pipeline} />}
      {currentView === "settings" && <SettingsView />}
      {currentView === "onboarding" && <OnboardingView />}
    </AppShell>
  );
}

export default App;
