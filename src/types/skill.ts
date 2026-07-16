export type SkillScope = "global" | "workspace";

export type SkillSource = "builtin" | "user" | "imported";

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
  enabled: boolean;
  boundAgentIds: string[];
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
}

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
