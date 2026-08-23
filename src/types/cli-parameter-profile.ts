import type { ManagedCliAgentId } from "./agent";
import type {
  CliArgumentSlot,
  CliLaunchScope,
  CliParameterDefinition,
  CliParameterSelections,
} from "./cli-parameter";

export type CliParameterSupport =
  | { state: "supported" }
  | { state: "not-installed" }
  | { state: "unknown-version"; requiredRange?: string }
  | { state: "unsupported-version"; installedVersion: string; requiredRange: string }
  | { state: "unsupported-platform"; platform: string };

export interface CliInstallationSnapshot {
  installed: boolean;
  runnable: boolean;
  activePath?: string;
  version?: string;
  conflict: boolean;
}

export interface CliArgumentToken {
  value: string;
  parameterId: string;
  segment: CliArgumentSlot;
}

export interface CliArgumentSegments {
  global: CliArgumentToken[];
  invocation: CliArgumentToken[];
}

export type CliParameterDiagnosticCode =
  | "LEGACY_SELECTION_MIGRATED"
  | "LEGACY_SELECTION_QUARANTINED"
  | "UNSUPPORTED_BY_ACTIVE_VERSION"
  | "UNSUPPORTED_PLATFORM"
  | "UNSUPPORTED_VALUE"
  | "VERSION_UNKNOWN"
  | "CLI_NOT_INSTALLED"
  | "ACTIVE_INSTALLATION_CONFLICT"
  | "DEPENDENCY_NOT_SATISFIED"
  | "CONFLICTING_SELECTION"
  | "MODEL_DEPENDENT_VALUE"
  | "MISSING_DIRECTORY"
  | "CATALOG_REVIEW_REQUIRED"
  | "REVISION_CONFLICT"
  | "CATALOG_VERSION_CONFLICT";

export type CliParameterDiagnosticSeverity = "error" | "warning" | "info";

export type CliParameterRemediation =
  | "none"
  | "repair-selection"
  | "adjust-dependency"
  | "reselect-directory"
  | "reload-profile"
  | "open-cli-management";

export interface CliParameterDiagnostic {
  code: CliParameterDiagnosticCode;
  severity: CliParameterDiagnosticSeverity;
  agentId: ManagedCliAgentId;
  parameterId?: string;
  messageKey: string;
  blocking: boolean;
  remediation: CliParameterRemediation;
  details?: Record<string, string>;
}

export interface CliParameterFieldView {
  definition: CliParameterDefinition;
  support: CliParameterSupport;
  optionSupport: Record<string, CliParameterSupport>;
}

export interface CliParameterSavedPreviews {
  chat: CliArgumentSegments;
  interactive: CliArgumentSegments;
}

export interface CliParameterProfile {
  agentId: ManagedCliAgentId;
  catalogVersion: string;
  revision: number;
  updatedAt: string | null;
  installation: CliInstallationSnapshot;
  fields: CliParameterFieldView[];
  selections: CliParameterSelections;
  savedPreviews: CliParameterSavedPreviews;
  diagnostics: CliParameterDiagnostic[];
}

export interface PreviewCliParameterProfileInput {
  agentId: ManagedCliAgentId;
  catalogVersion: string;
  scope: CliLaunchScope;
  selections: CliParameterSelections;
  /** Echoed back so a slower response can be discarded without server-side UI knowledge. */
  requestId?: string;
}

export interface CliParameterPreview {
  agentId: ManagedCliAgentId;
  catalogVersion: string;
  scope: CliLaunchScope;
  requestId?: string;
  normalizedSelections: CliParameterSelections;
  segments: CliArgumentSegments;
  diagnostics: CliParameterDiagnostic[];
}

export interface SaveCliParameterProfileInput {
  agentId: ManagedCliAgentId;
  expectedRevision: number;
  catalogVersion: string;
  selections: CliParameterSelections;
}

export interface ResetCliParameterProfileInput {
  agentId: ManagedCliAgentId;
  expectedRevision: number;
  catalogVersion: string;
}

export type CliParameterErrorCode =
  | "CLI_PARAMETER_UNKNOWN_AGENT"
  | "CLI_PARAMETER_UNKNOWN_PARAMETER"
  | "CLI_PARAMETER_INVALID_VALUE"
  | "CLI_PARAMETER_DEPENDENCY_UNSATISFIED"
  | "CLI_PARAMETER_CONFLICT"
  | "CLI_PARAMETER_UNSUPPORTED_VERSION"
  | "CLI_PARAMETER_REVISION_CONFLICT"
  | "CLI_PARAMETER_CATALOG_MISMATCH"
  | "CLI_PARAMETER_CATALOG_INVALID"
  | "CLI_PARAMETER_REPOSITORY_FAILURE";

/** Structured, machine-readable failure. The page maps `code` to localized text and never parses
 * backend prose. */
export interface CliParameterServiceError {
  code: CliParameterErrorCode;
  agentId?: ManagedCliAgentId;
  parameterId?: string;
  details?: Record<string, string>;
}

// The service-error guard and the diagnostic/support helpers land with the draft engine in
// `upgrade-cli-parameter-management` section 11. This module stays type-only until then.
