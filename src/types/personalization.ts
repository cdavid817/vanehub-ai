/**
 * The personalization wire contract, mirroring `src-tauri/src/commands/personalization/dto.rs`
 * field for field.
 *
 * The unions below are the closed sets the native side renders today. Where two of them spell a
 * multi-word value differently -- `rebuilding_derived` but `repair-required` -- that is the native
 * rendering reproduced verbatim, not a typo to normalise on one side: a screen matching on a
 * string the other end never sends is silently dead code.
 *
 * What no shape here carries: a memory body in a list entry, an absolute path, a legacy folder, a
 * remote URI, a core prompt, or a raw persistence error.
 */

export type PolicyScopeKind = "global" | "agent" | "workspace" | "workspace-agent";

export type PolicyToggle = "inherit" | "enabled" | "disabled";

export type InstructionMergeMode = "inherit" | "append" | "replace" | "disabled";

export type MemoryScopeKind = "global" | "workspace";

/** `untyped` is a migration outcome that can be read back, never one a user may choose. */
export type MemoryType = "user" | "feedback" | "project" | "reference" | "untyped";

export type MemoryStatus = "candidate" | "active" | "archived";

export type MemorySource =
  | "explicit_user"
  | "onepiece_automatic"
  | "cli_automatic"
  | "model_memory_tool"
  | "legacy_migration"
  | "external_file_edit";

export type MemorySensitivity = "normal" | "sensitive";

export type SessionPersonalizationMode = "standard" | "project-only" | "temporary";

export type PersonalizationHealthState =
  | "not_started"
  | "busy"
  | "migrating"
  | "rebuilding_derived"
  | "ready"
  | "repair_required"
  | "failed";

export interface PersonalizationHealth {
  state: PersonalizationHealthState;
  memoryAvailable: boolean;
  pendingCandidates: number;
}

export interface PersonalizationPolicy {
  scopeKind: PolicyScopeKind;
  /** The Agent id, the workspace key, or both joined. Empty for global. */
  scopeKey: string;
  revision: number;
  instructionMergeMode: InstructionMergeMode;
  aboutUser: string;
  styleRules: string;
  memoryReadMode: PolicyToggle;
  explicitSaveMode: PolicyToggle;
  automaticExtractionMode: PolicyToggle;
  globalMemoryAccessMode: PolicyToggle;
}

export interface PersonalizationPolicyRef {
  scopeKind: PolicyScopeKind;
  agentId?: string;
  workspaceKey?: string;
}

/**
 * One layer's edit. Every field beyond the scope is optional because a screen posts what the user
 * touched -- sending all of them would republish the ones they did not, which is how one screen's
 * stale copy silently reverts another's.
 */
export interface PersonalizationPolicyPatch extends PersonalizationPolicyRef {
  /** Absent creates the layer; present requires it to still be at that revision. */
  expectedRevision?: number;
  instructionMergeMode?: InstructionMergeMode;
  aboutUser?: string;
  styleRules?: string;
  memoryReadMode?: PolicyToggle;
  explicitSaveMode?: PolicyToggle;
  automaticExtractionMode?: PolicyToggle;
  globalMemoryAccessMode?: PolicyToggle;
}

/**
 * What one Agent can actually consume, so a screen renders its controls from the Agent rather than
 * from a list of Agent ids it carries itself.
 */
export interface AgentPersonalizationCapability {
  agentId: string;
  displayName: string;
  supportsCustomInstructions: boolean;
  supportsMemoryIndex: boolean;
  supportsSelectedMemoryBodies: boolean;
  supportsAutomaticExtraction: boolean;
}

export interface EffectivePreviewInput {
  agentId: string;
  sessionId: string;
  workspaceKey?: string;
  workspaceDisplayPath?: string;
  sessionMode?: SessionPersonalizationMode;
}

export type InstructionField = "about_user" | "style_rules";

export type InstructionMergeAction = "appended" | "replaced";

export type InstructionExclusionReason =
  | "empty_field"
  | "replaced_by_higher_layer"
  | "disabled_by_higher_layer"
  | "inherited_layer"
  | "runtime_capability";

/** One instruction field as it would be applied, already redacted. */
export interface PreviewSegment {
  field: InstructionField;
  scopeKind: PolicyScopeKind;
  scopeKey: string;
  policyRevision: number;
  mergeAction: InstructionMergeAction;
  /**
   * Redacted through the same rule the logs use. A settings screen gets screenshotted into issues,
   * so a token a user pasted into their own instructions is not handed back.
   */
  redactedText: string;
  /** Length of the real text, not of the redaction: a user sizing their instructions needs it. */
  characters: number;
}

export interface ExcludedSegment {
  field: InstructionField;
  scopeKind: PolicyScopeKind;
  scopeKey: string;
  reason: InstructionExclusionReason;
}

export type MemoryDelivery = "none" | "index_only" | "index_with_selected_bodies";

export type MemoryExclusionReason =
  | "project_only_session"
  | "temporary_session"
  | "other_workspace"
  | "agent_audience"
  | "pending_candidate"
  | "archived"
  | "global_memory_disabled"
  | "memory_read_disabled"
  | "runtime_capability"
  | "unsafe_maintenance_state";

export interface MemoryExclusion {
  reason: MemoryExclusionReason;
  count: number;
}

export type PersonalizationWarning =
  | "using-last-known-good-policy"
  | "no-validated-policy"
  | "migration-incomplete"
  | "repair-required"
  | "unsupported-capability-override"
  | "unknown-agent"
  | "workspace-required";

export interface EffectivePreview {
  revisionToken: string;
  instructionMode: InstructionMergeMode;
  includedInstructions: PreviewSegment[];
  excludedInstructions: ExcludedSegment[];
  memoryDelivery: MemoryDelivery;
  memoryRead: boolean;
  explicitSave: boolean;
  automaticExtraction: boolean;
  candidateCreation: boolean;
  retrievalWrite: boolean;
  eligibleMemoryCount: number;
  consideredMemoryCount: number;
  memoryExclusions: MemoryExclusion[];
  warnings: PersonalizationWarning[];
  approximateTokens: number;
  knownCharacters: number;
  selectedBodyBudgetMax: number;
  excludedSurfaces: string[];
  estimatorVersion: string;
  /**
   * Always false, and reported rather than assumed: VaneHub does not manage a CLI's internal
   * context, so a screen that stayed silent would leave a user thinking the estimate covers their
   * whole session.
   */
  cliInternalCompactionManaged: boolean;
}

/**
 * What a caller knows about a workspace, in the forms the native side can identify one from.
 *
 * A remote workspace travels as its parts, never as a URI: a URI can carry `user:password@host`,
 * and parts have nowhere to put one -- so a credential cannot reach the boundary by accident.
 */
export interface WorkspaceScopeInput {
  /** A stable id the workspace subsystem already assigns; preferred over anything derived. */
  stableId?: string;
  projectPath?: string;
  /** A worktree is its own workspace, so it wins over the project it was cut from. */
  worktreePath?: string;
  remote?: {
    host: string;
    port?: number;
    user?: string;
    path: string;
  };
}

/** The key alone: the caller already has a name for the workspace it just described. */
export interface WorkspaceScope {
  workspaceKey: string;
  kind: "local" | "remote";
}
