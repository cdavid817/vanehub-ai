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
   * False while the page is mounted but hidden. Most pages unmount instead of ever seeing this go
   * false (`keepAlive: "never"`, settings-page-lifecycle.ts's default per design.md Decision 6) —
   * this only matters for the pages with a documented reason to stay mounted while inactive, whose
   * background work has to be gated on this rather than on mount.
   */
  isActive: boolean;
  /**
   * Reports (or clears, with `null`) the active page's own unsaved-draft state so the shell can
   * guard navigation away from changes its own lifecycle would otherwise silently lose (task
   * 12.12). Carries a count and two callbacks, never the field values themselves, so a secret
   * draft value structurally cannot reach the shell, a route, storage, or a log through this path
   * — the owning page is the only thing that ever holds it. A page whose `saveMode` is not
   * "draft"/"mixed", or that has nothing dirty right now, simply never calls this with a
   * non-null value.
   */
  onDraftStateChange?: (guard: SettingsDraftGuard | null) => void;
  /**
   * Reports (or clears, with `null`) the page's own current status for its nav entry's bounded
   * semantic indicator (task 12.16, spec.md "Show page status"). Unlike `onDraftStateChange`,
   * this is offered to every currently *rendered* page, not only the active one, so a
   * backgrounded `draft-only` page (task 12.17) can keep flagging itself while the user looks at
   * something else; a `never` page simply has nothing left to report once it unmounts, same as
   * everything else about it. A page with more than one true condition at once should combine
   * them with `pickPageStatus` (`settings-page-status.ts`) rather than invent its own priority.
   */
  onStatusChange?: (status: SettingsPageStatus | null) => void;
}

export interface SettingsDraftGuard {
  /** For copy like "3 unsaved changes will be lost" in the shell's own leave prompt. */
  dirtyCount: number;
  /** False when the draft can't be saved right now (local validation error or a server-side
   *  conflict) -- the shell still offers Discard/Stay, with Save disabled rather than hidden,
   *  mirroring `DraftActionBar`'s own `saveDisabled`. */
  canSave: boolean;
  save: () => Promise<void> | void;
  discard: () => void;
}

/** design.md "Show page status": the five conditions a nav entry MAY flag, one at a time. */
export type SettingsPageStatusKind =
  | "error"
  | "dependency-unavailable"
  | "unsaved"
  | "restart-required"
  | "update-available";

export interface SettingsPageStatus {
  kind: SettingsPageStatusKind;
  /** Localized text for the indicator's accessible description -- never the raw condition name,
   *  and never a value from the page's own data (a status is a shape, not a secret). */
  labelKey: string;
  labelParams?: Record<string, string | number>;
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

/**
 * One searchable field within a page (task 12.1/12.4-12.6). `anchorId` must match a real
 * `id` rendered in that page's DOM (typically a `SettingsRow`'s own row id) -- the search
 * navigator scrolls/focuses it, so a field with no rendered anchor cannot be listed here.
 */
export interface SettingsSearchField {
  id: string;
  labelKey: string;
  /** Extra synonyms beyond the localized label itself (task 12.1's "keywords", scenario "Search a synonym"). */
  keywords?: string[];
  anchorId: string;
}

/** design.md Decision 17: "immediate" saves per row, "draft" batches through a shared
 *  DraftActionBar, "mixed" pages declare a page-specific split between the two. */
export type SettingsSaveMode = "immediate" | "draft" | "mixed";

/** design.md Decision 17 / spec.md "Settings danger and sensitivity hierarchy": "sensitive"
 *  pages hold credentials or other secrets: "dangerous" pages expose destructive actions
 *  (reset/uninstall/disconnect/revoke/erase) needing consequence-aware confirmation. */
export type SettingsRiskLevel = "normal" | "sensitive" | "dangerous";

export interface SettingsPageDefinition {
  id: SettingsPageId;
  labelKey: string;
  crumbKey: string;
  group: SettingsPageGroup;
  icon: LucideIcon;
  badge?: number;
  searchPlaceholderKey: string;
  /** Bounded, localized summary shown under the label in search results (task 12.5). */
  descriptionKey: string;
  /** Extra page-level search synonyms beyond `labelKey`/`descriptionKey` themselves. */
  keywords: string[];
  /** Empty is valid and honest for a page whose fields have not been indexed yet -- the page
   *  still matches on label/description/keywords, it just contributes no field-level hits. */
  fields: SettingsSearchField[];
  saveMode: SettingsSaveMode;
  risk: SettingsRiskLevel;
  loader: LazyFeatureLoader<SettingsPageContext>;
}
