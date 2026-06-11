import type { LucideIcon } from "lucide-react";
import {
  BarChart3,
  BookOpen,
  Braces,
  History,
  Layers,
  Mic,
  Settings,
} from "lucide-react";
import type { AppView } from "../../lib/views";

interface SidebarProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
}

interface NavItem {
  id: AppView;
  label: string;
  icon: LucideIcon;
}

const iconProps = {
  size: 18,
  strokeWidth: 1.5,
  absoluteStrokeWidth: true,
} as const;

const primaryItems: NavItem[] = [
  { id: "main", label: "Accueil", icon: Mic },
  { id: "dictionary", label: "Dictionnaire", icon: BookOpen },
  { id: "snippets", label: "Snippets", icon: Braces },
  { id: "context", label: "Contexte", icon: Layers },
  { id: "history", label: "Historique", icon: History },
  { id: "insight", label: "Insight", icon: BarChart3 },
];

const bottomItems: NavItem[] = [
  { id: "settings", label: "Paramètres", icon: Settings },
];

function NavButton({
  item,
  active,
  onNavigate,
}: {
  item: NavItem;
  active: boolean;
  onNavigate: (view: AppView) => void;
}) {
  const Icon = item.icon;

  return (
    <button
      type="button"
      onClick={() => onNavigate(item.id)}
      className={[
        "group relative flex w-full items-center gap-3 rounded-md px-3 py-2",
        "font-[family-name:var(--font-body)] text-sm font-medium tracking-wide",
        "transition-colors duration-150",
        active
          ? "border border-hairline-strong bg-surface-elevated text-ink"
          : "border border-transparent text-body hover:text-ink",
      ].join(" ")}
      aria-current={active ? "page" : undefined}
    >
      {active && (
        <span
          className="absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-full bg-accent-blue"
          aria-hidden
        />
      )}
      <Icon
        {...iconProps}
        className={[
          "shrink-0 transition-colors duration-150",
          active ? "text-accent-blue" : "text-charcoal group-hover:text-ink",
        ].join(" ")}
        aria-hidden
      />
      <span>{item.label}</span>
    </button>
  );
}

export function Sidebar({ currentView, onNavigate }: SidebarProps) {
  return (
    <aside className="relative flex w-[220px] shrink-0 flex-col border-r border-hairline bg-canvas">
      <div
        className="pointer-events-none absolute inset-x-0 top-0 h-32"
        style={{
          background:
            "radial-gradient(ellipse 80% 100% at 50% 0%, var(--color-accent-blue-glow) 0%, transparent 70%)",
          opacity: 0.08,
        }}
        aria-hidden
      />

      <div className="relative px-5 pb-4 pt-6">
        <button
          type="button"
          onClick={() => onNavigate("main")}
          className="text-display-serif text-2xl text-ink transition-opacity hover:opacity-80"
        >
          Calliop
        </button>
      </div>

      <nav
        className="relative flex flex-1 flex-col gap-1 px-3"
        aria-label="Navigation principale"
      >
        {primaryItems.map((item) => (
          <NavButton
            key={item.id}
            item={item}
            active={currentView === item.id}
            onNavigate={onNavigate}
          />
        ))}
      </nav>

      <div className="relative mt-auto flex flex-col gap-1 border-t border-hairline px-3 py-4">
        {bottomItems.map((item) => (
          <NavButton
            key={item.id}
            item={item}
            active={currentView === item.id}
            onNavigate={onNavigate}
          />
        ))}
      </div>
    </aside>
  );
}
