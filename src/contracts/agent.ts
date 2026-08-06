export type InteractionMode = "browser" | "native-desktop" | "cli" | "api";

export type AvailabilityState =
  "available" | "unavailable" | "needs-auth" | "unknown";

export type AgentOrigin = "builtin" | "user";

export type SessionLifecycleState =
  "idle" | "starting" | "running" | "failed" | "stopped";

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

export interface WorkflowState {
  activeAgentId: string | null;
  activeInteractionMode: InteractionMode | null;
  lifecycleState: SessionLifecycleState;
  intent: string;
}

/** One participant in a session: an Agent playing an expert role. */
export interface SessionSeat {
  agentId: string;
  /** Null for a plain single-Agent session, which has no role assigned. */
  roleId: string | null;
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

export const managedCliAgentIds = [
  "claude-code",
  "codex-cli",
  "gemini-cli",
  "opencode",
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
export type {
  RemoteCommandRun,
  RemoteCommandTemplate,
  RemoteCommandRunStatus,
  RemoteCommandTemplateScope,
  RemoteHostKeyChallenge,
  RemoteOutputChunk,
  RemoteOutputSearchQuery,
  RemoteOutputSearchResult,
  RemoteTerminalBinding,
  RemoteTerminalEndpoint,
  RemoteTerminalState,
  RemoteTerminalStatus,
} from "../types/remote-terminal";
