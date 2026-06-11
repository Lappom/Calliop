import type { LucideIcon } from "lucide-react";
import {
  BarChart3,
  BookOpen,
  Braces,
  History,
  Home,
  Palette,
  Settings,
} from "lucide-react";
import type { AppView } from "../../lib/views";

export interface NavItem {
  id: AppView;
  label: string;
  icon: LucideIcon;
}

export interface NavSection {
  label?: string;
  items: NavItem[];
}

export const NAV_SECTIONS: NavSection[] = [
  {
    items: [{ id: "main", label: "Accueil", icon: Home }],
  },
  {
    label: "Activité",
    items: [
      { id: "history", label: "Historique", icon: History },
      { id: "insight", label: "Statistiques", icon: BarChart3 },
    ],
  },
  {
    label: "Personnalisation",
    items: [
      { id: "dictionary", label: "Dictionnaire", icon: BookOpen },
      { id: "snippets", label: "Snippets", icon: Braces },
      { id: "style", label: "Style", icon: Palette },
    ],
  },
];

export const BOTTOM_NAV_ITEMS: NavItem[] = [
  { id: "settings", label: "Paramètres", icon: Settings },
];
