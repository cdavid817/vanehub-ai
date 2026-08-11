export type JsonPrimitive = boolean | number | string | null;
export type JsonValue = JsonPrimitive | JsonValue[] | JsonObject;
export interface JsonObject { [key: string]: JsonValue }

export const lspLanguageIds: readonly ["rust", "typescript_javascript"] = [
  "rust", "typescript_javascript",
];
export type LspLanguageId = (typeof lspLanguageIds)[number];

export const lspServerKinds: readonly ["rust_analyzer", "typescript_language_server"] = [
  "rust_analyzer", "typescript_language_server",
];
export type LspServerKind = (typeof lspServerKinds)[number];

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
] = [
  "executable_not_found", "override_missing", "override_not_executable",
  "executable_unavailable", "minimal_project_failed", "spawn_failed",
  "initialize_failed", "initialize_timed_out", "forced_termination", "cleanup_failed",
  "invalid_deadline", "restart_exhausted", "protocol_limit", "request_timeout",
  "cancelled", "untrusted", "unsupported_method", "invalid_configuration",
];
export type LspSafeReasonCode = (typeof lspSafeReasonCodes)[number];

export interface LspLanguageConfiguration {
  language: LspLanguageId;
  enabled: boolean;
  executableOverride: string | null;
  initializationOptions: JsonObject;
}

export interface LspConfiguration {
  enabled: boolean;
  languages: LspLanguageConfiguration[];
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

export interface LspNegotiatedCapabilities {
  positionEncoding: LspPositionEncoding;
  documentSync: LspDocumentSyncMode;
  definition: boolean;
  references: boolean;
  hover: boolean;
  diagnostics: boolean;
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

export const lspToolNames: readonly [
  "find_definition", "find_references", "get_hover", "get_diagnostics",
] = ["find_definition", "find_references", "get_hover", "get_diagnostics"];
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

export type LspToolResult =
  | { metadata: LspToolResultMetadata; definitions: LspToolLocation[] }
  | { metadata: LspToolResultMetadata; references: LspToolLocation[] }
  | { metadata: LspToolResultMetadata; hover: LspToolHover | null }
  | { metadata: LspToolResultMetadata; diagnostics: LspToolDiagnostic[] };
