import {
  BarChart3,
  Boxes,
  Cpu,
  Info,
  KeyRound,
  MessagesSquare,
  Activity,
  Puzzle,
  Plug,
  Settings,
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  Terminal,
  UserRound,
  Workflow,
  type LucideIcon,
} from "lucide-react";
import type { LazyFeatureLoader } from "../components/lazy-feature";
import type { CliConfigAgentId } from "../types/cli-agent-config";

export type SettingsPageId =
  | "basic"
  | "providers"
  | "cli-parameters"
  | "extensions"
  | "plugins"
  | "mcp"
  | "agent-configurations"
  | "expert-roles"
  | "agent-policies"
  | "personalization"
  | "skills"
  | "prompt-hooks"
  | "im"
  | "ssh-connections"
  | "observability"
  | "usage"
  | "about";

export interface SettingsPageContext {
  searchTerm: string;
  navigationTarget: SettingsNavigationTarget | null;
  onNavigate: (pageId: SettingsPageId, target?: SettingsNavigationTarget) => void;
  onReturn?: () => void;
  /**
   * False while the page is mounted but hidden. Visited pages stay mounted so their state survives
   * tab switches, which means background work has to be gated on this rather than on mount.
   */
  isActive: boolean;
}

export interface SettingsNavigationTarget {
  cliConfigAgentId?: CliConfigAgentId;
  agentConfigAgentId?: CliConfigAgentId | "onepiece";
}

export interface SettingsPageDefinition {
  id: SettingsPageId;
  labelKey: string;
  crumbKey: string;
  icon: LucideIcon;
  badge?: number;
  searchPlaceholderKey: string;
  loader: LazyFeatureLoader<SettingsPageContext>;
}

const loadBasicPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/basic-settings-page")
  .then((module) => ({ default: module.BasicSettingsPage }));
const loadProvidersPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/providers-page")
  .then((module) => ({ default: module.ProvidersPage }));
const loadCliParametersPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/cli-parameters-page")
  .then((module) => ({ default: module.CliParametersPage }));
const loadExtensionsPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/extensions-page")
  .then((module) => ({ default: module.ExtensionsPage }));
const loadMcpPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/mcp-page")
  .then((module) => ({ default: module.McpPage }));
const loadAgentConfigurationsPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/agent-configurations-page")
  .then((module) => ({ default: module.AgentConfigurationsPage }));
const loadExpertRolesPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/expert-roles-page")
  .then((module) => ({ default: module.ExpertRolesPage }));
const loadAgentPoliciesPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/agent-policies-page")
  .then((module) => ({ default: module.AgentPoliciesPage }));
const loadPersonalizationPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/personalization-page")
  .then((module) => ({ default: module.PersonalizationPage }));
const loadSkillsPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/skills-page")
  .then((module) => ({ default: module.SkillsPage }));
const loadPromptHooksPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/prompt-hooks-page")
  .then((module) => ({ default: module.PromptHooksPage }));
const loadImPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/im-page")
  .then((module) => ({ default: module.ImPage }));
const loadSshConnectionsPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/ssh-connections-page")
  .then((module) => ({ default: module.SshConnectionsPage }));
const loadPluginIntegrationsPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/plugin-integrations-page")
  .then((module) => ({ default: module.PluginIntegrationsPage }));
const loadObservabilityPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/observability-settings-page")
  .then((module) => ({ default: module.ObservabilitySettingsPage }));
const loadUsagePage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/usage-statistics-page")
  .then((module) => ({ default: module.UsageStatisticsPage }));
const loadAboutPage: LazyFeatureLoader<SettingsPageContext> = () => import("./pages/about-page")
  .then((module) => ({ default: module.AboutPage }));

export const settingsPages: SettingsPageDefinition[] = [
  {
    id: "basic",
    labelKey: "settings.pages.basic",
    crumbKey: "settings.pages.basic",
    icon: Settings,
    searchPlaceholderKey: "settings.search.basic",
    loader: loadBasicPage,
  },
  {
    id: "agent-configurations",
    labelKey: "settings.pages.agentConfigurations",
    crumbKey: "settings.pages.agentConfigurations",
    icon: Settings2,
    searchPlaceholderKey: "settings.search.agentConfigurations",
    loader: loadAgentConfigurationsPage,
  },
  {
    id: "agent-policies",
    labelKey: "settings.pages.agentPolicies",
    crumbKey: "settings.pages.agentPolicies",
    icon: ShieldCheck,
    searchPlaceholderKey: "settings.search.agentPolicies",
    loader: loadAgentPoliciesPage,
  },
  {
    id: "cli-parameters",
    labelKey: "settings.pages.cliParameters",
    crumbKey: "settings.pages.cliParameters",
    icon: SlidersHorizontal,
    searchPlaceholderKey: "settings.search.cliParameters",
    loader: loadCliParametersPage,
  },
  {
    id: "mcp",
    labelKey: "settings.pages.mcp",
    crumbKey: "settings.pages.mcp",
    icon: Boxes,
    searchPlaceholderKey: "settings.search.mcp",
    loader: loadMcpPage,
  },
  {
    id: "skills",
    labelKey: "settings.pages.skills",
    crumbKey: "settings.pages.skills",
    icon: Puzzle,
    searchPlaceholderKey: "settings.search.skills",
    loader: loadSkillsPage,
  },
  {
    id: "personalization",
    labelKey: "settings.pages.personalization",
    crumbKey: "settings.pages.personalization",
    icon: Sparkles,
    searchPlaceholderKey: "settings.search.personalization",
    loader: loadPersonalizationPage,
  },
  {
    id: "prompt-hooks",
    labelKey: "settings.pages.promptHooks",
    crumbKey: "settings.pages.promptHooks",
    icon: Workflow,
    searchPlaceholderKey: "settings.search.promptHooks",
    loader: loadPromptHooksPage,
  },
  {
    id: "expert-roles",
    labelKey: "settings.pages.expertRoles",
    crumbKey: "settings.pages.expertRoles",
    icon: UserRound,
    searchPlaceholderKey: "settings.search.expertRoles",
    loader: loadExpertRolesPage,
  },
  {
    id: "providers",
    labelKey: "settings.pages.providers",
    crumbKey: "settings.pages.providers",
    icon: Terminal,
    searchPlaceholderKey: "settings.search.providers",
    loader: loadProvidersPage,
  },
  {
    id: "extensions",
    labelKey: "settings.pages.extensions",
    crumbKey: "settings.pages.extensions",
    icon: Cpu,
    searchPlaceholderKey: "settings.search.extensions",
    loader: loadExtensionsPage,
  },
  {
    id: "plugins",
    labelKey: "settings.pages.plugins",
    crumbKey: "settings.pages.plugins",
    icon: Plug,
    searchPlaceholderKey: "settings.search.plugins",
    loader: loadPluginIntegrationsPage,
  },
  {
    id: "im",
    labelKey: "settings.pages.im",
    crumbKey: "settings.pages.im",
    icon: MessagesSquare,
    searchPlaceholderKey: "settings.search.im",
    loader: loadImPage,
  },
  {
    id: "ssh-connections",
    labelKey: "settings.pages.sshConnections",
    crumbKey: "settings.pages.sshConnections",
    icon: KeyRound,
    searchPlaceholderKey: "settings.search.sshConnections",
    loader: loadSshConnectionsPage,
  },
  {
    id: "observability",
    labelKey: "settings.pages.observability",
    crumbKey: "settings.pages.observability",
    icon: Activity,
    searchPlaceholderKey: "settings.search.observability",
    loader: loadObservabilityPage,
  },
  {
    id: "usage",
    labelKey: "settings.pages.usage",
    crumbKey: "settings.pages.usage",
    icon: BarChart3,
    searchPlaceholderKey: "settings.search.usage",
    loader: loadUsagePage,
  },
  {
    id: "about",
    labelKey: "settings.pages.about",
    crumbKey: "settings.pages.about",
    icon: Info,
    searchPlaceholderKey: "settings.search.about",
    loader: loadAboutPage,
  },
];

export const defaultSettingsPageId: SettingsPageId = "basic";

export function getSettingsPage(id: SettingsPageId) {
  return settingsPages.find((page) => page.id === id) ?? settingsPages[0];
}
