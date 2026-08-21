import {
  BarChart3,
  BookOpen,
  Boxes,
  Braces,
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
import {
  loadAboutPage,
  loadAgentConfigurationsPage,
  loadAgentPoliciesPage,
  loadBasicPage,
  loadCliParametersPage,
  loadCodeIntelligencePage,
  loadDocumentationPage,
  loadExpertRolesPage,
  loadExtensionsPage,
  loadImPage,
  loadMcpPage,
  loadObservabilityPage,
  loadPersonalizationPage,
  loadPluginIntegrationsPage,
  loadPromptHooksPage,
  loadProvidersPage,
  loadSkillsPage,
  loadSshConnectionsPage,
  loadUsagePage,
} from "./settings-page-loaders";
import type { CliConfigAgentId } from "../types/cli-agent-config";

export type SettingsPageId =
  | "basic"
  | "providers"
  | "cli-parameters"
  | "extensions"
  | "plugins"
  | "mcp"
  | "agent-configurations"
  | "code-intelligence"
  | "expert-roles"
  | "agent-policies"
  | "personalization"
  | "skills"
  | "prompt-hooks"
  | "im"
  | "ssh-connections"
  | "observability"
  | "usage"
  | "help"
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

/**
 * Sidebar grouping only. The page order itself is a deliberate product decision asserted by
 * `tests/e2e/settings-navigation-order.spec.ts`, so groups must stay contiguous in that order
 * rather than re-sorting pages into a tidier taxonomy.
 */
export type SettingsPageGroup = "general" | "agent" | "capabilities" | "integrations" | "diagnostics";

export const settingsPageGroupOrder: SettingsPageGroup[] = [
  "general",
  "agent",
  "capabilities",
  "integrations",
  "diagnostics",
];

export interface SettingsPageDefinition {
  id: SettingsPageId;
  labelKey: string;
  crumbKey: string;
  group: SettingsPageGroup;
  icon: LucideIcon;
  badge?: number;
  searchPlaceholderKey: string;
  loader: LazyFeatureLoader<SettingsPageContext>;
}

export const settingsPages: SettingsPageDefinition[] = [
  {
    id: "basic",
    group: "general",
    labelKey: "settings.pages.basic",
    crumbKey: "settings.pages.basic",
    icon: Settings,
    searchPlaceholderKey: "settings.search.basic",
    loader: loadBasicPage,
  },
  {
    id: "agent-configurations",
    group: "agent",
    labelKey: "settings.pages.agentConfigurations",
    crumbKey: "settings.pages.agentConfigurations",
    icon: Settings2,
    searchPlaceholderKey: "settings.search.agentConfigurations",
    loader: loadAgentConfigurationsPage,
  },
  {
    id: "agent-policies",
    group: "agent",
    labelKey: "settings.pages.agentPolicies",
    crumbKey: "settings.pages.agentPolicies",
    icon: ShieldCheck,
    searchPlaceholderKey: "settings.search.agentPolicies",
    loader: loadAgentPoliciesPage,
  },
  {
    id: "cli-parameters",
    group: "agent",
    labelKey: "settings.pages.cliParameters",
    crumbKey: "settings.pages.cliParameters",
    icon: SlidersHorizontal,
    searchPlaceholderKey: "settings.search.cliParameters",
    loader: loadCliParametersPage,
  },
  {
    id: "code-intelligence",
    group: "capabilities",
    labelKey: "settings.pages.codeIntelligence",
    crumbKey: "settings.pages.codeIntelligence",
    icon: Braces,
    searchPlaceholderKey: "settings.search.codeIntelligence",
    loader: loadCodeIntelligencePage,
  },
  {
    id: "mcp",
    group: "capabilities",
    labelKey: "settings.pages.mcp",
    crumbKey: "settings.pages.mcp",
    icon: Boxes,
    searchPlaceholderKey: "settings.search.mcp",
    loader: loadMcpPage,
  },
  {
    id: "skills",
    group: "capabilities",
    labelKey: "settings.pages.skills",
    crumbKey: "settings.pages.skills",
    icon: Puzzle,
    searchPlaceholderKey: "settings.search.skills",
    loader: loadSkillsPage,
  },
  {
    id: "personalization",
    group: "capabilities",
    labelKey: "settings.pages.personalization",
    crumbKey: "settings.pages.personalization",
    icon: Sparkles,
    searchPlaceholderKey: "settings.search.personalization",
    loader: loadPersonalizationPage,
  },
  {
    id: "prompt-hooks",
    group: "capabilities",
    labelKey: "settings.pages.promptHooks",
    crumbKey: "settings.pages.promptHooks",
    icon: Workflow,
    searchPlaceholderKey: "settings.search.promptHooks",
    loader: loadPromptHooksPage,
  },
  {
    id: "expert-roles",
    group: "capabilities",
    labelKey: "settings.pages.expertRoles",
    crumbKey: "settings.pages.expertRoles",
    icon: UserRound,
    searchPlaceholderKey: "settings.search.expertRoles",
    loader: loadExpertRolesPage,
  },
  {
    id: "providers",
    group: "integrations",
    labelKey: "settings.pages.providers",
    crumbKey: "settings.pages.providers",
    icon: Terminal,
    searchPlaceholderKey: "settings.search.providers",
    loader: loadProvidersPage,
  },
  {
    id: "extensions",
    group: "integrations",
    labelKey: "settings.pages.extensions",
    crumbKey: "settings.pages.extensions",
    icon: Cpu,
    searchPlaceholderKey: "settings.search.extensions",
    loader: loadExtensionsPage,
  },
  {
    id: "plugins",
    group: "integrations",
    labelKey: "settings.pages.plugins",
    crumbKey: "settings.pages.plugins",
    icon: Plug,
    searchPlaceholderKey: "settings.search.plugins",
    loader: loadPluginIntegrationsPage,
  },
  {
    id: "im",
    group: "integrations",
    labelKey: "settings.pages.im",
    crumbKey: "settings.pages.im",
    icon: MessagesSquare,
    searchPlaceholderKey: "settings.search.im",
    loader: loadImPage,
  },
  {
    id: "ssh-connections",
    group: "integrations",
    labelKey: "settings.pages.sshConnections",
    crumbKey: "settings.pages.sshConnections",
    icon: KeyRound,
    searchPlaceholderKey: "settings.search.sshConnections",
    loader: loadSshConnectionsPage,
  },
  {
    id: "observability",
    group: "diagnostics",
    labelKey: "settings.pages.observability",
    crumbKey: "settings.pages.observability",
    icon: Activity,
    searchPlaceholderKey: "settings.search.observability",
    loader: loadObservabilityPage,
  },
  {
    id: "usage",
    group: "diagnostics",
    labelKey: "settings.pages.usage",
    crumbKey: "settings.pages.usage",
    icon: BarChart3,
    searchPlaceholderKey: "settings.search.usage",
    loader: loadUsagePage,
  },
  {
    id: "help",
    group: "diagnostics",
    labelKey: "settings.pages.help",
    crumbKey: "settings.pages.help",
    icon: BookOpen,
    searchPlaceholderKey: "settings.search.help",
    loader: loadDocumentationPage,
  },
  {
    id: "about",
    group: "diagnostics",
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
