// Runtime constants mirror the committed contract fixture and are safe to consume outside React.
export const SKILL_MANAGEMENT_SCOPES = ["global", "workspace"] as const;
export const SKILL_RUNTIME_LAYERS = ["project", "user", "registry", "system"] as const;
export const SKILL_FIXED_READ_TOOLS = ["list_skills", "load_skill", "read_skill_resource"] as const;
export const SKILL_LOGICAL_URI_PREFIX = "skill://" as const;
export const SKILL_LEGACY_COMPATIBILITY_DEFAULTS = { type: "role", delivery: "eager" } as const;

export type SkillScope = (typeof SKILL_MANAGEMENT_SCOPES)[number];

export type SkillSource = "builtin" | "user" | "imported";

export type SkillType = "role" | "utility";

export type SkillDelivery = "eager" | "on-demand";

export type SkillLayer = (typeof SKILL_RUNTIME_LAYERS)[number];

export type SkillOrigin = "created" | "imported" | "installed" | "shipped" | "migrated";

export type SkillTrust = "trusted" | "untrusted";

export type SkillAvailability = "available" | "disabled" | "invalid" | "conflicting" | "unsupported";

export interface SkillDelegationCapability {
  supported: boolean;
  reason: "available" | "native-runtime-unavailable" | "not-utility" | "skill-unavailable";
}

export interface SkillScopeInput {
  scope: SkillScope;
  workspacePath?: string | null;
}

export interface SkillMetadata {
  id: string;
  name: string;
  description: string;
  category: string;
  version: string;
  triggers: string[];
  aliases?: string[];
  type?: SkillType;
  delivery?: SkillDelivery;
  compatibilityDefaults?: SkillCompatibilityDefaults;
}

export interface SkillCompatibilityDefaults {
  skillType: boolean;
  delivery: boolean;
}

export interface SkillShadowSummary {
  layer: SkillLayer;
  origin: SkillOrigin;
  version: string;
  availability: SkillAvailability;
}

export interface SkillUsageSummary {
  viewCount: number;
  useCount: number;
  lastViewedAt: string | null;
  lastUsedAt: string | null;
  revisionWitness: string | null;
}

export interface SkillAgentBinding {
  agentId: string;
  mountPath: string;
  mountedPath: string;
  mounted: boolean;
}

export interface Skill {
  id: string;
  scope: SkillScope;
  workspacePath: string | null;
  source: SkillSource;
  enabled: boolean;
  skillDir: string;
  skillMdPath: string;
  contentHash: string;
  metadata: SkillMetadata;
  boundAgentIds: string[];
  bindings: SkillAgentBinding[];
  createdAt: string;
  updatedAt: string;
  layer: SkillLayer;
  origin: SkillOrigin;
  trust: SkillTrust;
  availability: SkillAvailability;
  delegationCapability?: SkillDelegationCapability;
  immutable: boolean;
  shadowedDefinitions: SkillShadowSummary[];
  usage: SkillUsageSummary;
}

export interface SkillStats {
  total: number;
  enabled: number;
  mounted: number;
}

export interface SkillListResult {
  skills: Skill[];
  stats: SkillStats;
}

export interface SkillCompatibleAgent {
  id: string;
  displayName: string;
  kind: "cli" | "api";
}

export interface SkillOverview {
  skills: Skill[];
  stats: SkillStats;
  mountPaths: SkillAgentMountPath[];
  agents: SkillCompatibleAgent[];
  apiAgentBindings: Record<string, string[]>;
  drift: SkillDriftReport;
  restoreCandidates: string[];
}

export interface SkillAgentMountPath {
  agentId: string;
  mountPath: string;
  isDefault: boolean;
}

export interface SkillMutationInput extends SkillScopeInput {
  id: string;
  metadata: SkillMetadata;
  body: string;
  enabled: boolean;
  boundAgentIds: string[];
  source?: SkillSource;
}

export interface SkillUpdateInput extends SkillScopeInput {
  metadata: SkillMetadata;
  body: string;
  expectedContentHash: string;
}

export interface SkillImportInput extends SkillScopeInput {
  sourcePath: string;
  enabled: boolean;
  boundAgentIds: string[];
}

export interface SkillPreview extends SkillScopeInput {
  id: string;
  content: string;
  path: string;
  layer: SkillLayer;
  origin: SkillOrigin;
  availability: SkillAvailability;
  immutable: boolean;
  shadowedDefinitions: SkillShadowSummary[];
}

export interface SkillLoadInput {
  idOrAlias: string;
  workspacePath?: string | null;
}

export interface SkillResourceReadInput {
  uri: string;
  revision: string;
  workspacePath?: string | null;
}

export interface SkillResourceEntry {
  uri: string;
  relativePath: string;
  sizeBytes: number;
}

export interface SkillResourceIndex {
  scripts: SkillResourceEntry[];
  references: SkillResourceEntry[];
  templates: SkillResourceEntry[];
  assets: SkillResourceEntry[];
  truncated: boolean;
}

export interface SkillLoadResult {
  id: string;
  name: string;
  content: string;
  truncated: boolean;
  revision: string;
  baseUri: string;
  resources: SkillResourceIndex;
}

export interface SkillResourceReadResult {
  id: string;
  uri: string;
  revision: string;
  content: string;
  sizeBytes: number;
}

export type SkillAccessRefusalReason =
  | "not-found"
  | "ambiguous-alias"
  | "disabled"
  | "invalid"
  | "conflicting"
  | "unsupported"
  | "utility-not-loadable"
  | "invalid-uri"
  | "unindexed-resource"
  | "stale-revision"
  | "escaping-resource"
  | "binary-resource"
  | "oversized-resource"
  | "unreadable";

export interface SkillAccessRefusal {
  requested: string;
  canonicalId: string | null;
  reason: SkillAccessRefusalReason;
  conflictingIds: string[];
}

export type SkillLoadOutcome =
  | { status: "loaded"; result: SkillLoadResult }
  | { status: "refused"; refusal: SkillAccessRefusal };

export type SkillResourceReadOutcome =
  | { status: "read"; result: SkillResourceReadResult }
  | { status: "refused"; refusal: SkillAccessRefusal };

export type SkillDriftIssueType =
  | "missing-source"
  | "metadata-changed"
  | "unregistered-source"
  | "missing-mount"
  | "conflict"
  | "deleted-builtin";

export interface SkillDriftIssue {
  skillId: string;
  type: SkillDriftIssueType;
  agentId?: string | null;
  path?: string | null;
  message: string;
}

export interface SkillDriftReport extends SkillScopeInput {
  issues: SkillDriftIssue[];
  driftHash: string;
}

export interface SkillBackupEntry {
  originalPath: string;
  backupPath: string;
}

export interface SkillSyncResult {
  mounted: string[];
  unmounted: string[];
  overwritten: string[];
  backedUp: SkillBackupEntry[];
  restored: string[];
  failed: Array<{ skillId: string; reason: string }>;
  resolvedFrom: SkillDriftReport;
}

export interface SkillMountMigrationReport {
  agentId: string;
  oldMountPath: string;
  newMountPath: string;
  migrated: string[];
  removed: string[];
  overwritten: string[];
  backedUp: SkillBackupEntry[];
  failed: Array<{ skillId: string; reason: string }>;
}

export type * from "./skill-overlay";
export type * from "./skill-overlay-reconciliation";
