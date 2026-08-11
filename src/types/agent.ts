export type InteractionMode = "browser" | "native-desktop" | "cli" | "api";

export type AvailabilityState =
  "available" | "unavailable" | "needs-auth" | "unknown";

export type AgentOrigin = "builtin" | "user";

export type SessionLifecycleState =
  "idle" | "starting" | "running" | "failed" | "stopped";

export type SessionRecoveryStatus =
  "clean" | "reconciling" | "action_required" | "quarantined";

export type ImSessionConnector =
  "feishu" | "telegram" | "dingtalk" | "wecom" | "weixin";

export interface SessionSourceMetadata {
  kind: "desktop" | "im";
  connector: ImSessionConnector | null;
}

export interface LaunchMetadata {
  kind: "cli" | "browser" | "desktop" | "api";
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
  agentOrigin: AgentOrigin;
}

export type ApiInterfaceFormat = "anthropic" | "openai-compatible";
export type ProviderEndpointType =
  | "anthropic-messages"
  | "openai-chat-completions"
  | "openai-responses";
export type ProviderAuthStrategy = "x-api-key" | "bearer";

export interface RegisterApiAgentInput {
  displayName: string;
  provider: string;
  apiKey: string;
  modelId: string;
  interfaceFormat: ApiInterfaceFormat;
  baseUrl: string | null;
}

export interface ApiAgentProviderConfig {
  modelId: string;
  interfaceFormat: ApiInterfaceFormat;
  baseUrl: string | null;
  autoApproveTools: boolean;
}

export interface OnePieceProviderConfig {
  provider: string;
  modelId: string | null;
  interfaceFormat: ApiInterfaceFormat | null;
  baseUrl: string | null;
  autoApproveTools: boolean;
  credentialPresent: boolean;
}

export interface SaveOnePieceProviderConfigInput {
  provider: string;
  modelId: string;
  interfaceFormat: ApiInterfaceFormat;
  baseUrl: string | null;
  apiKey?: string | null;
}

export interface OnePieceProviderProfile {
  id: string;
  name: string;
  sourceProviderId: string | null;
  sourceEndpointType: ProviderEndpointType | null;
  sourcePresetVersion: number | null;
  provider: string;
  modelId: string;
  interfaceFormat: ApiInterfaceFormat;
  baseUrl: string | null;
  active: boolean;
  credentialPresent: boolean;
}

export interface OnePieceProviderPreset {
  id: string;
  catalogVersion: number;
  displayName: string;
  category: "official" | "common";
  iconKey: string;
  provider: string;
  defaultModelId: string;
  fallbackModels: string[];
  interfaceFormat: ApiInterfaceFormat;
  baseUrl: string | null;
  apiKeyUrl: string;
  docsUrl: string;
  defaultEndpointType: ProviderEndpointType;
  endpoints: OnePieceProviderEndpoint[];
  modelDiscovery: {
    strategy: "anthropic" | "openai" | "openai-array" | "catalog";
  };
}

export interface OnePieceProviderEndpoint {
  type: ProviderEndpointType;
  baseUrl: string;
  interfaceFormat: ApiInterfaceFormat;
  authStrategy: ProviderAuthStrategy;
  source: string;
  modelDiscovery: {
    strategy: "anthropic" | "openai" | "openai-array" | "catalog";
    url: string | null;
  };
}

export interface OnePieceProviderProfiles {
  profiles: OnePieceProviderProfile[];
  activeProfileId: string | null;
}

export interface DiscoverOnePieceProviderModelsInput {
  providerId: string;
  endpointType: ProviderEndpointType;
  profileId?: string | null;
  apiKey?: string | null;
}

export interface ValidateOnePieceProviderCredentialInput {
  providerId: string;
  endpointType: ProviderEndpointType;
  modelId: string;
  profileId?: string | null;
  apiKey?: string | null;
}

export interface OnePieceProviderModelOption {
  id: string;
  displayName: string;
  source: "api" | "catalog" | "profile";
}

export interface OnePieceProviderModelDiscoveryResult {
  providerId: string;
  endpointType: ProviderEndpointType;
  models: OnePieceProviderModelOption[];
  source: "merged" | "catalog";
  warning: "live-unavailable" | null;
}

export interface SaveOnePieceProviderProfileInput {
  id?: string | null;
  name: string;
  providerId: string;
  endpointType: ProviderEndpointType;
  modelId: string;
  apiKey?: string | null;
}

export interface UpdateApiAgentInput {
  displayName: string;
  modelId: string;
  baseUrl: string | null;
  newApiKey?: string | null;
}

export type AgentMemorySource = "explicit" | "automatic";

export interface AgentMemory {
  id: string;
  agentId: string;
  folder: string | null;
  content: string;
  source: AgentMemorySource;
  createdAt: string;
}

// `add-retrieval-vector-search` §7.4: configuration is a global singleton, while index status
// and rebuild are scoped per agent and aggregate across all of that agent's `scope_folder` rows.
export interface RetrievalConfiguration {
  sourceProfileId: string | null;
  embeddingModel: string | null;
  automaticCodeIndexMode: import("./code-index").CodeIndexAutomaticMode;
}

export interface RetrievalIndexStatus {
  indexed: number;
  pending: number;
  failed: number;
  // Category only (e.g. "auth" | "invalid_request" | "rate_limit" | "network") — never raw error
  // text, which may carry credentials or provider response content (design doc §8.2).
  lastFailureCategory: string | null;
}

export interface EmbeddingModelOption {
  id: string;
  displayName: string;
}

export interface WorkflowState {
  activeAgentId: string | null;
  activeInteractionMode: InteractionMode | null;
  lifecycleState: SessionLifecycleState;
  intent: string;
}

export interface SessionSeatRoleSnapshot {
  roleName: string | null;
  avatar: string;
  color: string;
  responsibility: string | null;
  agentName: string;
  modelFamily: "anthropic" | "openai" | "google" | "unknown";
  crossFamilyReviewer: boolean;
}

/** One participant in a session: an Agent playing an expert role. */
export interface SessionSeat {
  /** Assigned by the session service. Creation inputs may omit it. */
  seatId?: string;
  agentId: string;
  /** Null for a plain single-Agent session, which has no role assigned. */
  roleId: string | null;
  /** Captured when the participant joins so role edits cannot rewrite history. */
  roleSnapshot?: SessionSeatRoleSnapshot | null;
  joinedAt?: string;
  /** Departed participants remain available for historical message attribution. */
  leftAt?: string | null;
}

export interface UpdateSessionSeatsInput {
  sessionId: string;
  expectedUpdatedAt: string;
  seats: SessionSeat[];
}

export interface Session {
  id: string;
  title: string;
  agentId: string;
  /**
   * Optional because sessions predate seats. When absent the session is a one-seat session whose
   * seat is `agentId`; when present, `agentId` mirrors `seats[0].agentId`.
   */
  seats?: SessionSeat[];
  interactionMode: InteractionMode;
  lifecycleState: SessionLifecycleState;
  recoveryStatus: SessionRecoveryStatus;
  recoveryRevision: number;
  stateRevision: number;
  historyRevision: number;
  activeExecutionRunId: string | null;
  folder: string | null;
  projectPath: string | null;
  worktreePath: string | null;
  worktreeName: string | null;
  worktreeBranch: string | null;
  remoteWorkspace: RemoteWorkspace | null;
  remoteSshConnectionId: string | null;
  remoteSshConnectionRevision: number | null;
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

export type ScheduledTaskFrequency =
  | { kind: "minutes"; interval: number }
  | { kind: "hours"; interval: number }
  | { kind: "daily"; timeOfDay: string }
  | { kind: "weekly"; weekday: number; timeOfDay: string }
  | { kind: "monthly"; dayOfMonth: number; timeOfDay: string };

export type ScheduledTaskLatestStatus =
  "never-run" | "running" | "succeeded" | "failed" | "skipped";

export interface ScheduledTask {
  id: string;
  name: string;
  content: string;
  agentId: string;
  frequency: ScheduledTaskFrequency;
  enabled: boolean;
  nextRunAt: string;
  latestStatus: ScheduledTaskLatestStatus;
  latestRunAt: string | null;
  latestRunSessionId: string | null;
  latestError: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateScheduledTaskInput {
  name: string;
  content: string;
  agentId: string;
  frequency: ScheduledTaskFrequency;
}

export interface SetScheduledTaskEnabledInput {
  taskId: string;
  enabled: boolean;
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

export interface RemoteWorkspace {
  host: string;
  port?: number | null;
  user: string | null;
  path: string;
  displayName: string;
  uri: string;
}

export interface KnownRemoteWorkspace extends RemoteWorkspace {
  lastOpenedAt: string;
}

export interface CreateSessionInput {
  agentId: string;
  /**
   * Omitted for a single-Agent session, which the native layer records as one seat built from
   * `agentId`. When present, `agentId` must equal `seats[0].agentId`.
   */
  seats?: SessionSeat[];
  interactionMode: InteractionMode;
  title?: string;
  folder?: string | null;
  projectPath?: string | null;
  remoteWorkspace?: {
    host: string;
    port?: number | null;
    user?: string | null;
    path: string;
    displayName?: string | null;
    sshConnectionId?: string | null;
  } | null;
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
  adapter: "browser" | "native-desktop" | "cli" | "api" | "none";
  details: Record<string, string>;
}

export type AgentTerminalState = "starting" | "running" | "stopped" | "failed";
export type AgentTerminalCapability = "native" | "simulated";

export interface AgentTerminalSize {
  rows: number;
  cols: number;
}

export interface AgentTerminalSession {
  terminalId: string;
  sessionId: string;
  agentId: string;
  state: AgentTerminalState;
  capability: AgentTerminalCapability;
  size: AgentTerminalSize;
  runtimeSessionId: string | null;
  retained: boolean;
}

export type AgentTerminalEvent =
  | { type: "output"; terminalId: string; sessionId: string; content: string }
  | {
      type: "state";
      terminalId: string;
      sessionId: string;
      state: AgentTerminalState;
      error: string | null;
    }
  | {
      type: "runtime_session_id";
      terminalId: string;
      sessionId: string;
      runtimeSessionId: string;
    };

export type CliVersionCheckStatus =
  "unsupported" | "not-detected" | "succeeded" | "failed";
export type CliEnvironmentType = "windows" | "macos" | "linux" | "unknown";
export type CliInstallSource =
  | "npm"
  | "winget"
  | "desktop"
  | "homebrew"
  | "volta"
  | "bun"
  | "vendor"
  | "system"
  | "unknown";
export type CliConflictState =
  "none" | "multiple" | "version-mismatch" | "runnable-mismatch";
export type CliLifecycleEligibility =
  "npm" | "wget" | "winget" | "manual" | "unavailable";

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
  /** Null for CLIs distributed only by installer script, which have no npm package. */
  packageName: string | null;
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

export const managedCliAgentIds = [
  "claude-code",
  "codex-cli",
  "gemini-cli",
  "opencode",
  "antigravity-cli",
] as const;
export type ManagedCliAgentId = (typeof managedCliAgentIds)[number];
export type CliParameterControl = "enum" | "boolean" | "multi-enum" | "custom-text";
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
