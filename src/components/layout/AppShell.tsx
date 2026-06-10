import type { ReactNode } from "react";
import type { AppView } from "../../lib/views";
import { NavBar } from "./NavBar";

interface AppShellProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  children: ReactNode;
}

export function AppShell({ currentView, onNavigate, children }: AppShellProps) {
  return (
    <div className="flex min-h-screen flex-col bg-canvas text-body">
      <NavBar currentView={currentView} onNavigate={onNavigate} />
      <main className="mx-auto w-full max-w-[720px] flex-1 px-8 py-8">
        {children}
      </main>
    </div>
  );
}
