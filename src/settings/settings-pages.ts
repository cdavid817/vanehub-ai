import {
  Bot,
  BarChart3,
  Boxes,
  Code2,
  Cpu,
  Info,
  MessagesSquare,
  Puzzle,
  Plug,
  Settings,
  SlidersHorizontal,
  Terminal,
  type LucideIcon,
} from "lucide-react";
import { AgentsPage } from "./pages/agents-page";
import { AboutPage } from "./pages/about-page";
import { BasicSettingsPage } from "./pages/basic-settings-page";
import { CliParametersPage } from "./pages/cli-parameters-page";
import { ExtensionsPage } from "./pages/extensions-page";
import { McpPage } from "./pages/mcp-page";
import { ImPage } from "./pages/im-page";
import { PluginIntegrationsPage } from "./pages/plugin-integrations-page";
import { ProvidersPage } from "./pages/providers-page";
import { SdkPage } from "./pages/sdk-page";
import { SkillsPage } from "./pages/skills-page";
import { UsageStatisticsPage } from "./pages/usage-statistics-page";

export type SettingsPageId =
  | "basic"
  | "providers"
  | "cli-parameters"
  | "sdk"
  | "extensions"
  | "plugins"
  | "mcp"
  | "agents"
  | "skills"
  | "im"
  | "usage"
  | "about";

export interface SettingsPageContext {
  searchTerm: string;
  onNavigate: (pageId: SettingsPageId) => void;
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
    icon: Terminal,
    badge: 4,
    searchPlaceholderKey: "settings.search.providers",
    component: ProvidersPage,
  },
  {
    id: "cli-parameters",
    labelKey: "settings.pages.cliParameters",
    crumbKey: "settings.pages.cliParameters",
    icon: SlidersHorizontal,
    badge: 4,
    searchPlaceholderKey: "settings.search.cliParameters",
    component: CliParametersPage,
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
    id: "extensions",
    labelKey: "settings.pages.extensions",
    crumbKey: "settings.pages.extensions",
    icon: Cpu,
    badge: 3,
    searchPlaceholderKey: "settings.search.extensions",
    component: ExtensionsPage,
  },
  {
    id: "plugins",
    labelKey: "settings.pages.plugins",
    crumbKey: "settings.pages.plugins",
    icon: Plug,
    badge: 1,
    searchPlaceholderKey: "settings.search.plugins",
    component: PluginIntegrationsPage,
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
  {
    id: "im",
    labelKey: "settings.pages.im",
    crumbKey: "settings.pages.im",
    icon: MessagesSquare,
    badge: 5,
    searchPlaceholderKey: "settings.search.im",
    component: ImPage,
  },
  {
    id: "usage",
    labelKey: "settings.pages.usage",
    crumbKey: "settings.pages.usage",
    icon: BarChart3,
    searchPlaceholderKey: "settings.search.usage",
    component: UsageStatisticsPage,
  },
  {
    id: "about",
    labelKey: "settings.pages.about",
    crumbKey: "settings.pages.about",
    icon: Info,
    searchPlaceholderKey: "settings.search.about",
    component: AboutPage,
  },
];

export const defaultSettingsPageId: SettingsPageId = "basic";

export function getSettingsPage(id: SettingsPageId) {
  return settingsPages.find((page) => page.id === id) ?? settingsPages[0];
}
