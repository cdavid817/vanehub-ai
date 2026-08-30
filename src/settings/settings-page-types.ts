import type { LucideIcon } from "lucide-react";
import type { LazyFeatureLoader } from "../components/lazy-feature";
import type { CliConfigAgentId } from "../types/cli-agent-config";

/**
 * The shape of the settings navigation, split out from the page list itself.
 *
 * Two reasons for the split. The page list is where product decisions live and it has to stay
 * readable at a glance, and the loaders module needs `SettingsPageContext` without importing the
 * list -- taking the type from here instead of from `settings-pages.ts` removes the last reason
 * that import had to be type-only.
 */
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
  | "local-media"
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
  /** Opens one session in the workspace. Separate from `onReturn` because returning is about
   * leaving settings and this is about arriving somewhere specific; a caller that has no route to
   * a session simply does not pass it, and the surfaces that offer the link hide it. */
  onOpenSession?: (sessionId: string) => void;
  /**
   * False while the page is mounted but hidden. Visited pages stay mounted so their state survives
   * tab switches, which means background work has to be gated on this rather than on mount.
   */
  isActive: boolean;
}

export interface SettingsNavigationTarget {
  cliConfigAgentId?: CliConfigAgentId;
  agentConfigAgentId?: CliConfigAgentId | "onepiece";
  curatorCandidateId?: string;
  curatorWorkspaceId?: string;
  generationWorkspaceId?: string;
  generationJobId?: string;
  evolutionWorkspaceId?: string;
  evolutionRunId?: string;
  evolutionApplicationId?: string;
  evolutionProbationId?: string;
  evolutionBreakerId?: string;
  overlayHistoryId?: string;
  overlaySkillId?: string;
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
