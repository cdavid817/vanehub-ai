export type InteractionMode = "browser" | "native-desktop" | "cli";

export type AvailabilityState = "available" | "unavailable" | "needs-auth" | "unknown";

export type SessionLifecycleState = "idle" | "starting" | "running" | "failed" | "stopped";

export type ImSessionConnector = "feishu" | "telegram" | "dingtalk" | "wecom" | "weixin";

export interface SessionSourceMetadata {
  kind: "desktop" | "im";
  connector: ImSessionConnector | null;
}

export interface LaunchMetadata {
  kind: "cli" | "browser" | "desktop";
  command?: string;
  url?: string;
  executableName?: string;
}

export interface AgentRegistryEntry {
  id: string;
  displayName: string;
  provider: string;
  managedSdkDependencyId?: string | null;
  launch: LaunchMetadata;
  supportedInteractionModes: InteractionMode[];
  availabilityState: AvailabilityState;
  unavailableReason?: string;
  capabilityTags: string[];
}

export interface WorkflowState {
  activeAgentId: string | null;
  activeInteractionMode: InteractionMode | null;
  lifecycleState: SessionLifecycleState;
  intent: string;
}

export interface Session {
  id: string;
  title: string;
  agentId: string;
  interactionMode: InteractionMode;
  lifecycleState: SessionLifecycleState;
  folder: string | null;
  projectPath: string | null;
  worktreePath: string | null;
  worktreeName: string | null;
  worktreeBranch: string | null;
  runtimeSessionId: string | null;
  categoryId: string | null;
  source?: SessionSourceMetadata;
  pinned: boolean;
  archived: boolean;
  createdAt: string;
  updatedAt: string;
}

export type SessionSearchMatchKind = "title" | "project" | "message";

export interface SessionSearchMatch {
  kind: SessionSearchMatchKind;
  excerpt: string;
  messageId?: string | null;
}

export interface SessionSearchInput {
  query: string;
  limit?: number;
}

export interface SessionSearchResult {
  session: Session;
  matches: SessionSearchMatch[];
}

export interface SessionCategory {
  id: string;
  name: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface CreateSessionCategoryInput {
  name: string;
}

export interface RenameSessionCategoryInput {
  categoryId: string;
  name: string;
}

export interface AssignSessionCategoryInput {
  sessionId: string;
  categoryId: string | null;
}

export type SessionExportFormat = "json" | "markdown";

export interface ExportSessionInput {
  sessionId: string;
  format: SessionExportFormat;
  destinationDirectory?: string | null;
}

export type SessionExportStatus = "exported" | "cancelled" | "unavailable";

export interface SessionExportResult {
  status: SessionExportStatus;
  path?: string | null;
  content?: string | null;
}

export interface AutomaticArchivalSettings {
  enabled: boolean;
  inactiveDays: number;
}

export interface KnownProject {
  path: string;
  displayName: string;
  isGit: boolean;
  lastOpenedAt: string;
}

export interface ProjectInspection {
  path: string;
  displayName: string;
  isGit: boolean;
  gitRoot: string | null;
}

export interface CreateSessionInput {
  agentId: string;
  interactionMode: InteractionMode;
  title?: string;
  folder?: string | null;
  projectPath?: string | null;
  worktree?: {
    enabled: boolean;
    name?: string;
  } | null;
}

export interface ReadinessStatus {
  ready: boolean;
  reason?: string;
  requiresAuthentication: boolean;
}

export interface LaunchResult {
  operationId?: string | null;
  workflow: WorkflowState;
  message: string;
}

export interface SessionDetails {
  agentId: string | null;
  interactionMode: InteractionMode | null;
  lifecycleState: SessionLifecycleState;
  adapter: "browser" | "native-desktop" | "cli" | "none";
  details: Record<string, string>;
}

export type CliVersionCheckStatus = "unsupported" | "not-detected" | "succeeded" | "failed";
export type CliEnvironmentType = "windows" | "macos" | "linux" | "unknown";
export type CliInstallSource = "npm" | "winget" | "desktop" | "homebrew" | "volta" | "bun" | "vendor" | "system" | "unknown";
export type CliConflictState = "none" | "multiple" | "version-mismatch" | "runnable-mismatch";
export type CliLifecycleEligibility = "npm" | "wget" | "winget" | "manual" | "unavailable";

export interface CliInstallation {
  path: string;
  version: string | null;
  runnable: boolean;
  error: string | null;
  source: CliInstallSource;
  environmentType: CliEnvironmentType;
  isActive: boolean;
}

export interface CliToolStatus {
  agentId: string;
  displayName: string;
  provider: string;
  executableName: string;
  packageName: string;
  installed: boolean | null;
  currentVersion: string | null;
  latestVersion: string | null;
  availableVersions: string[];
  detectedPath: string | null;
  installCommand: string;
  lastCheckedAt: string | null;
  lastError: string | null;
  lastOperationId: string | null;
  versionCheckStatus: CliVersionCheckStatus;
  environmentType: CliEnvironmentType;
  installations: CliInstallation[];
  activeInstallationPath: string | null;
  conflictState: CliConflictState;
  lifecycleEligibility: CliLifecycleEligibility;
}

export interface CliPackageOperationInput {
  agentId: string;
  targetVersion: string;
  confirmedActivePath?: string | null;
}

export const managedCliAgentIds = ["claude-code", "codex-cli", "gemini-cli", "opencode"] as const;
export type ManagedCliAgentId = (typeof managedCliAgentIds)[number];
export type CliParameterControl = "enum" | "boolean" | "multi-enum";
export type CliParameterValue = string | boolean | string[];
export type CliParameterLaunchScope = "interactive" | "chat";
export type CliParameterRisk = "normal" | "warning";

export interface CliParameterOption {
  value: string;
  labelKey: string;
  descriptionKey: string;
}

export interface CliParameterDefinition {
  id: string;
  agentId: ManagedCliAgentId;
  flag: string;
  control: CliParameterControl;
  labelKey: string;
  descriptionKey: string;
  options: CliParameterOption[];
  defaultValue: CliParameterValue;
  launchScopes: CliParameterLaunchScope[];
  risk: CliParameterRisk;
}

export type CliParameterSelections = Record<string, CliParameterValue>;

export interface CliParameterProfile {
  agentId: ManagedCliAgentId;
  definitions: CliParameterDefinition[];
  selections: CliParameterSelections;
  previewArgs: string[];
}

export interface SaveCliParameterProfileInput {
  agentId: ManagedCliAgentId;
  selections: CliParameterSelections;
}
