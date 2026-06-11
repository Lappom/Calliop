import type { ReactNode } from "react";
import type { AppView } from "../../lib/views";
import { Sidebar } from "./Sidebar";

interface AppShellProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  children: ReactNode;
}

export function AppShell({ currentView, onNavigate, children }: AppShellProps) {
  return (
    <div className="flex min-h-screen bg-canvas text-body">
      <Sidebar currentView={currentView} onNavigate={onNavigate} />
      <main className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto w-full max-w-[880px]">{children}</div>
      </main>
    </div>
  );
}
