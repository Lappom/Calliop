import { useState } from "react";
import { AppShell } from "./components/layout/AppShell";
import { ContexteView } from "./components/context/ContexteView";
import { DictionaryView } from "./components/dictionary/DictionaryView";
import { HistoryView } from "./components/history/HistoryView";
import { InsightView } from "./components/insight/InsightView";
import { MainView } from "./components/main/MainView";
import { SettingsView } from "./components/settings/SettingsView";
import { SnippetsView } from "./components/snippets/SnippetsView";
import { usePipelineState } from "./hooks/usePipelineState";
import type { AppView } from "./lib/views";

function App() {
  const [currentView, setCurrentView] = useState<AppView>("main");
  const pipeline = usePipelineState();

  return (
    <AppShell currentView={currentView} onNavigate={setCurrentView}>
      {currentView === "main" && <MainView {...pipeline} />}
      {currentView === "dictionary" && <DictionaryView />}
      {currentView === "snippets" && <SnippetsView />}
      {currentView === "context" && <ContexteView />}
      {currentView === "history" && <HistoryView />}
      {currentView === "insight" && (
        <InsightView latencyMetrics={pipeline.latencyMetrics} />
      )}
      {currentView === "settings" && <SettingsView />}
    </AppShell>
  );
}

export default App;
