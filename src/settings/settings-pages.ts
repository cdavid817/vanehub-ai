import {
  Bot,
  Boxes,
  Code2,
  Database,
  Puzzle,
  Settings,
  type LucideIcon,
} from "lucide-react";
import { AgentsPage } from "./pages/agents-page";
import { BasicSettingsPage } from "./pages/basic-settings-page";
import { McpPage } from "./pages/mcp-page";
import { ProvidersPage } from "./pages/providers-page";
import { SdkPage } from "./pages/sdk-page";
import { SkillsPage } from "./pages/skills-page";

export type SettingsPageId = "basic" | "providers" | "sdk" | "mcp" | "agents" | "skills";

export interface SettingsPageContext {
  searchTerm: string;
}

export interface SettingsPageDefinition {
  id: SettingsPageId;
  labelKey: string;
  crumbKey: string;
  icon: LucideIcon;
  badge?: number;
  searchPlaceholderKey: string;
  component: (props: SettingsPageContext) => JSX.Element;
}

export const settingsPages: SettingsPageDefinition[] = [
  {
    id: "basic",
    labelKey: "settings.pages.basic",
    crumbKey: "settings.pages.basic",
    icon: Settings,
    searchPlaceholderKey: "settings.search.basic",
    component: BasicSettingsPage,
  },
  {
    id: "providers",
    labelKey: "settings.pages.providers",
    crumbKey: "settings.pages.providers",
    icon: Database,
    badge: 3,
    searchPlaceholderKey: "settings.search.providers",
    component: ProvidersPage,
  },
  {
    id: "sdk",
    labelKey: "settings.pages.sdk",
    crumbKey: "settings.pages.sdk",
    icon: Code2,
    badge: 5,
    searchPlaceholderKey: "settings.search.sdk",
    component: SdkPage,
  },
  {
    id: "mcp",
    labelKey: "settings.pages.mcp",
    crumbKey: "settings.pages.mcp",
    icon: Boxes,
    badge: 3,
    searchPlaceholderKey: "settings.search.mcp",
    component: McpPage,
  },
  {
    id: "agents",
    labelKey: "settings.pages.agents",
    crumbKey: "settings.pages.agents",
    icon: Bot,
    badge: 4,
    searchPlaceholderKey: "settings.search.agents",
    component: AgentsPage,
  },
  {
    id: "skills",
    labelKey: "settings.pages.skills",
    crumbKey: "settings.pages.skills",
    icon: Puzzle,
    badge: 8,
    searchPlaceholderKey: "settings.search.skills",
    component: SkillsPage,
  },
];

export const defaultSettingsPageId: SettingsPageId = "basic";

export function getSettingsPage(id: SettingsPageId) {
  return settingsPages.find((page) => page.id === id) ?? settingsPages[0];
}
