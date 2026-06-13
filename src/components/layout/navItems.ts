import type { TFunction } from "i18next";
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

export interface BottomNavItem {
  id: "settings";
  label: string;
  icon: LucideIcon;
}

export interface NavSection {
  label?: string;
  items: NavItem[];
}

export function getNavSections(t: TFunction): NavSection[] {
  return [
    {
      items: [{ id: "main", label: t("nav.items.main"), icon: Home }],
    },
    {
      label: t("nav.sections.activity"),
      items: [
        { id: "history", label: t("nav.items.history"), icon: History },
        { id: "insight", label: t("nav.items.insight"), icon: BarChart3 },
      ],
    },
    {
      label: t("nav.sections.personalization"),
      items: [
        { id: "dictionary", label: t("nav.items.dictionary"), icon: BookOpen },
        { id: "snippets", label: t("nav.items.snippets"), icon: Braces },
        { id: "style", label: t("nav.items.style"), icon: Palette },
      ],
    },
  ];
}

export function getBottomNavItems(t: TFunction): BottomNavItem[] {
  return [{ id: "settings", label: t("nav.items.settings"), icon: Settings }];
}
