export type JsonPrimitive = boolean | number | string | null;
export type JsonValue = JsonPrimitive | JsonValue[] | JsonObject;
export interface JsonObject { [key: string]: JsonValue }

/**
 * Which languages exist is a backend registry fact, so these are opaque ids rather than the
 * literal unions they used to be. The frontend learns the set from `LspConfiguration.descriptors`;
 * compiling a copy in here is what made adding a language a frontend change.
 */
export type LspLanguageId = string;
export type LspServerKind = string;

/** Shape both sides agree an id has, so a malformed one is refused rather than rendered. */
export const lspLanguageIdPattern = /^[a-z0-9_]{1,64}$/;

export const lspProcessStates: readonly [
  "absent", "starting", "initializing", "ready", "stopping", "backoff", "failed",
] = ["absent", "starting", "initializing", "ready", "stopping", "backoff", "failed"];
export type LspProcessState = (typeof lspProcessStates)[number];

export const lspDiscoveryAvailabilities: readonly ["available", "unavailable"] = [
  "available", "unavailable",
];
export type LspDiscoveryAvailability = (typeof lspDiscoveryAvailabilities)[number];

export const lspServerTestPhases: readonly ["discovery", "spawn", "initialize", "cleanup"] = [
  "discovery", "spawn", "initialize", "cleanup",
];
export type LspServerTestPhase = (typeof lspServerTestPhases)[number];

export const lspServerTestPhaseStatuses: readonly ["succeeded", "failed", "skipped"] = [
  "succeeded", "failed", "skipped",
];
export type LspServerTestPhaseStatus = (typeof lspServerTestPhaseStatuses)[number];

export const lspPositionEncodings: readonly ["utf8", "utf16"] = ["utf8", "utf16"];
export type LspPositionEncoding = (typeof lspPositionEncodings)[number];

export const lspDocumentSyncModes: readonly ["none", "full", "incremental"] = [
  "none", "full", "incremental",
];
export type LspDocumentSyncMode = (typeof lspDocumentSyncModes)[number];

export const lspSafeReasonCodes: readonly [
  "executable_not_found", "override_missing", "override_not_executable",
  "executable_unavailable", "minimal_project_failed", "spawn_failed",
  "initialize_failed", "initialize_timed_out", "forced_termination", "cleanup_failed",
  "invalid_deadline", "restart_exhausted", "protocol_limit", "request_timeout",
  "cancelled", "untrusted", "unsupported_method", "invalid_configuration",
  "unsupported_on_this_platform",
] = [
  "executable_not_found", "override_missing", "override_not_executable",
  "executable_unavailable", "minimal_project_failed", "spawn_failed",
  "initialize_failed", "initialize_timed_out", "forced_termination", "cleanup_failed",
  "invalid_deadline", "restart_exhausted", "protocol_limit", "request_timeout",
  "cancelled", "untrusted", "unsupported_method", "invalid_configuration",
  "unsupported_on_this_platform",
];
export type LspSafeReasonCode = (typeof lspSafeReasonCodes)[number];

export interface LspLanguageConfiguration {
  language: LspLanguageId;
  enabled: boolean;
  executableOverride: string | null;
  /** `null` means "use the registry default"; `[]` means the user chose no arguments. */
  startupArguments: string[] | null;
  initializationOptions: JsonObject;
}

/** What the backend registry declares about a language, so the UI renders no compiled-in list. */
export interface LspLanguageDescriptor {
  language: LspLanguageId;
  server: LspServerKind;
  supportedOnHost: boolean;
  defaultStartupArguments: string[];
}

export interface LspConfiguration {
  enabled: boolean;
  languages: LspLanguageConfiguration[];
  descriptors: LspLanguageDescriptor[];
}

export interface LspWorkspaceTrust {
  canonicalRoot: string;
  trusted: boolean;
  revision: number;
}

export interface LspWorkspaceTrustUpdate {
  canonicalRoot: string;
  trusted: boolean;
}

export interface LspServerDiscovery {
  language: LspLanguageId;
  server: LspServerKind;
  availability: LspDiscoveryAvailability;
  executablePath: string | null;
  arguments: string[];
  reasonCode: LspSafeReasonCode | null;
}

export interface LspServerTestInput { language: LspLanguageId }

export interface LspServerTestPhaseResult {
  phase: LspServerTestPhase;
  status: LspServerTestPhaseStatus;
  reasonCode: LspSafeReasonCode | null;
}

/** One method the backend implements, and whether this server advertised it. */
export interface LspNegotiatedMethod {
  method: string;
  supported: boolean;
}

export interface LspNegotiatedCapabilities {
  positionEncoding: LspPositionEncoding;
  documentSync: LspDocumentSyncMode;
  /**
   * One entry per method the backend implements, in the order it reports them. The frontend holds
   * no copy of that set, so a method added later renders without a change here.
   */
  methods: LspNegotiatedMethod[];
}

export interface LspServerTestResult {
  server: LspServerKind;
  phases: LspServerTestPhaseResult[];
  negotiatedCapabilities: LspNegotiatedCapabilities | null;
}

export interface LspServerStatus {
  language: LspLanguageId;
  server: LspServerKind;
  relativeProjectRoot: string;
  state: LspProcessState;
  restartCount: number;
  lastResponseAt: string | null;
  diagnosticCount: number;
  reasonCode: LspSafeReasonCode | null;
  negotiatedCapabilities: LspNegotiatedCapabilities | null;
}

/**
 * Declaration order mirrors the native catalog, which is append-only: a provider caches the
 * tool-definition prefix, so reordering what came before costs every eligible session its prompt
 * cache.
 */
export const lspToolNames: readonly [
  "find_definition", "find_references", "get_hover", "get_diagnostics",
  "find_type_definition", "find_implementations", "find_workspace_symbols",
  "get_document_symbols", "find_call_hierarchy",
] = [
  "find_definition", "find_references", "get_hover", "get_diagnostics",
  "find_type_definition", "find_implementations", "find_workspace_symbols",
  "get_document_symbols", "find_call_hierarchy",
];
export type LspToolName = (typeof lspToolNames)[number];

export type LspToolResultStatus = "ready" | "warming" | "timeout" | "unavailable" | "failed";

export interface LspToolResultMetadata {
  status: LspToolResultStatus;
  server: string | null;
  language: string | null;
  document_version: number | null;
  stale: boolean;
  returned_count: number;
  total: number;
  truncated: boolean;
  filtered_count: number;
  reason_code: string | null;
}

export interface LspToolRange {
  start_line: number;
  start_column: number;
  end_line: number;
  end_column: number;
}

export interface LspToolLocation {
  file: string;
  range: LspToolRange;
  preview: string | null;
}

export interface LspToolHover {
  signature: string | null;
  documentation: string | null;
  range: LspToolRange | null;
}

export interface LspToolDiagnostic {
  file: string;
  range: LspToolRange;
  severity: string | null;
  message: string;
  source: string | null;
  code: string | null;
}

export interface LspToolSymbol {
  name: string;
  kind: string;
  container: string | null;
  file: string;
  range: LspToolRange;
}

export interface LspToolCallRelation {
  symbol: LspToolSymbol;
  call_sites: LspToolRange[];
}

export type LspToolResult =
  | { metadata: LspToolResultMetadata; definitions: LspToolLocation[] }
  | { metadata: LspToolResultMetadata; references: LspToolLocation[] }
  | { metadata: LspToolResultMetadata; hover: LspToolHover | null }
  | { metadata: LspToolResultMetadata; diagnostics: LspToolDiagnostic[] }
  | { metadata: LspToolResultMetadata; symbols: LspToolSymbol[] }
  | { metadata: LspToolResultMetadata; relations: LspToolCallRelation[] };
