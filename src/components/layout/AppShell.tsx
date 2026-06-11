import { Menu, X } from "lucide-react";
import { useCallback, useEffect, useState, type ReactNode } from "react";
import type { AppView } from "../../lib/views";
import { Sidebar } from "./Sidebar";

interface AppShellProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  children: ReactNode;
}

export function AppShell({ currentView, onNavigate, children }: AppShellProps) {
  const [sidebarOpen, setSidebarOpen] = useState(false);

  const closeSidebar = useCallback(() => {
    setSidebarOpen(false);
  }, []);

  useEffect(() => {
    if (!sidebarOpen) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        closeSidebar();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [sidebarOpen, closeSidebar]);

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      <Sidebar
        currentView={currentView}
        onNavigate={onNavigate}
        open={sidebarOpen}
        onClose={closeSidebar}
      />

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-16 shrink-0 items-center gap-3 border-b border-hairline px-4 lg:hidden">
          <button
            type="button"
            onClick={() => setSidebarOpen((current) => !current)}
            className="inline-flex size-10 items-center justify-center rounded-md border border-hairline-strong bg-surface-elevated text-ink transition-colors hover:border-ink/30"
            aria-expanded={sidebarOpen}
            aria-label={sidebarOpen ? "Fermer le menu" : "Ouvrir le menu"}
          >
            {sidebarOpen ? <X size={20} /> : <Menu size={20} />}
          </button>
          <button
            type="button"
            onClick={() => onNavigate("main")}
            className="text-display-serif text-xl text-ink transition-opacity hover:opacity-80"
          >
            Calliop
          </button>
        </header>

        <main
          className="calliop-scroll flex-1 overflow-y-auto px-4 py-6 sm:px-6 sm:py-8 lg:px-8"
          inert={sidebarOpen ? true : undefined}
          aria-hidden={sidebarOpen ? true : undefined}
        >
          <div className="mx-auto w-full max-w-[880px]">{children}</div>
        </main>
      </div>
    </div>
  );
}
