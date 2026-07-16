export type InteractionMode = "browser" | "native-desktop" | "cli";

export type AvailabilityState = "available" | "unavailable" | "needs-auth" | "unknown";

export type SessionLifecycleState = "idle" | "starting" | "running" | "failed" | "stopped";

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
  pinned: boolean;
  archived: boolean;
  createdAt: string;
  updatedAt: string;
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
}

export interface CliPackageOperationInput {
  agentId: string;
  targetVersion: string;
}
