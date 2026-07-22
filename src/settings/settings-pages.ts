import {
  Bot,
  BarChart3,
  Boxes,
  Cpu,
  Info,
  KeyRound,
  MessagesSquare,
  Puzzle,
  Plug,
  Settings,
  SlidersHorizontal,
  Terminal,
  Workflow,
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
import { PromptHooksPage } from "./pages/prompt-hooks-page";
import { SkillsPage } from "./pages/skills-page";
import { SshConnectionsPage } from "./pages/ssh-connections-page";
import { UsageStatisticsPage } from "./pages/usage-statistics-page";

export type SettingsPageId =
  | "basic"
  | "providers"
  | "cli-parameters"
  | "extensions"
  | "plugins"
  | "mcp"
  | "agents"
  | "skills"
  | "prompt-hooks"
  | "im"
  | "ssh-connections"
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
    searchPlaceholderKey: "settings.search.providers",
    component: ProvidersPage,
  },
  {
    id: "cli-parameters",
    labelKey: "settings.pages.cliParameters",
    crumbKey: "settings.pages.cliParameters",
    icon: SlidersHorizontal,
    searchPlaceholderKey: "settings.search.cliParameters",
    component: CliParametersPage,
  },
  {
    id: "mcp",
    labelKey: "settings.pages.mcp",
    crumbKey: "settings.pages.mcp",
    icon: Boxes,
    searchPlaceholderKey: "settings.search.mcp",
    component: McpPage,
  },
  {
    id: "agents",
    labelKey: "settings.pages.agents",
    crumbKey: "settings.pages.agents",
    icon: Bot,
    searchPlaceholderKey: "settings.search.agents",
    component: AgentsPage,
  },
  {
    id: "skills",
    labelKey: "settings.pages.skills",
    crumbKey: "settings.pages.skills",
    icon: Puzzle,
    searchPlaceholderKey: "settings.search.skills",
    component: SkillsPage,
  },
  {
    id: "prompt-hooks",
    labelKey: "settings.pages.promptHooks",
    crumbKey: "settings.pages.promptHooks",
    icon: Workflow,
    searchPlaceholderKey: "settings.search.promptHooks",
    component: PromptHooksPage,
  },
  {
    id: "im",
    labelKey: "settings.pages.im",
    crumbKey: "settings.pages.im",
    icon: MessagesSquare,
    searchPlaceholderKey: "settings.search.im",
    component: ImPage,
  },
  {
    id: "ssh-connections",
    labelKey: "settings.pages.sshConnections",
    crumbKey: "settings.pages.sshConnections",
    icon: KeyRound,
    searchPlaceholderKey: "settings.search.sshConnections",
    component: SshConnectionsPage,
  },
  {
    id: "extensions",
    labelKey: "settings.pages.extensions",
    crumbKey: "settings.pages.extensions",
    icon: Cpu,
    searchPlaceholderKey: "settings.search.extensions",
    component: ExtensionsPage,
  },
  {
    id: "plugins",
    labelKey: "settings.pages.plugins",
    crumbKey: "settings.pages.plugins",
    icon: Plug,
    searchPlaceholderKey: "settings.search.plugins",
    component: PluginIntegrationsPage,
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
