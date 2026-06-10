import type { AppView } from "../../lib/views";
import { Button } from "../ui/Button";

interface NavBarProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
}

const navItems: { id: AppView; label: string }[] = [
  { id: "main", label: "Accueil" },
  { id: "settings", label: "Réglages" },
  { id: "onboarding", label: "Guide" },
];

export function NavBar({ currentView, onNavigate }: NavBarProps) {
  return (
    <header className="flex h-16 shrink-0 items-center justify-between border-b border-hairline px-8">
      <button
        type="button"
        onClick={() => onNavigate("main")}
        className="text-display-serif text-2xl text-ink transition-opacity hover:opacity-80"
      >
        Calliop
      </button>

      <nav
        className="hidden items-center gap-1 sm:flex"
        aria-label="Navigation principale"
      >
        {navItems.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => onNavigate(item.id)}
            className={[
              "rounded-full px-3.5 py-1.5",
              "font-[family-name:var(--font-body)] text-sm font-medium tracking-wide",
              "transition-colors duration-150",
              currentView === item.id
                ? "bg-surface-elevated text-ink"
                : "text-body hover:text-ink",
            ].join(" ")}
            aria-current={currentView === item.id ? "page" : undefined}
          >
            {item.label}
          </button>
        ))}
      </nav>

      <div className="flex items-center gap-3 sm:hidden">
        <select
          value={currentView}
          onChange={(e) => onNavigate(e.target.value as AppView)}
          className="rounded-md border border-hairline-strong bg-surface-card px-2 py-1 text-body-sm text-ink"
          aria-label="Choisir une section"
        >
          {navItems.map((item) => (
            <option key={item.id} value={item.id}>
              {item.label}
            </option>
          ))}
        </select>
      </div>

      <Button
        variant="outline"
        className="hidden sm:inline-flex"
        onClick={() => onNavigate("onboarding")}
      >
        Démarrer
      </Button>
    </header>
  );
}
