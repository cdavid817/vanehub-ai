import type {
  CliParameterPreview,
  CliParameterProfile,
  PreviewCliParameterProfileInput,
  ResetCliParameterProfileInput,
  SaveCliParameterProfileInput,
} from "../types/cli-parameter-profile";
import type {
  CliActionPlan,
  CliBulkActionPlan,
  CliEnvironmentSnapshot,
  ExecuteCliActionInput,
  PrepareCliActionInput,
} from "../types/cli-environment-snapshot";
import type {
  CliConfigDiscoveryResult,
  CliConfigPreset,
  CliConfigProfile,
  CliConfigStatus,
  DeleteCliConfigProfileInput,
  ImportCliConfigProfileInput,
  ImportDiscoveredCliConfigInput,
  ImportDiscoveredCliConfigResult,
  SaveCliConfigProfileInput,
  ValidateCliConfigCredentialInput,
} from "../types/cli-agent-config";
import type { OperationTask } from "../types/operation";
import type { ProviderCredentialValidationResult } from "../types/provider-credential-validation";

/**
 * The source-aware CLI environment boundary.
 *
 * Every method that can touch a process, a package manager, or the network returns an
 * `OperationTask`; the caller watches that operation rather than awaiting the work. The two
 * bounded reads -- the snapshot list and a persisted plan -- return their value directly, because
 * an operation id for something already stored would make the caller poll for a known answer.
 *
 * `prepareCliAction` takes what the user chose and `executeCliAction` takes only a plan id and the
 * revision the user saw. That split is the contract: there is no parameter on the execute call a
 * command could be rebuilt from, so the selected version cannot be dropped in between.
 */
export interface CliToolService {
  listCliEnvironments(): Promise<CliEnvironmentSnapshot[]>;
  refreshCliEnvironments(agentIds: string[], forceCatalog: boolean): Promise<OperationTask>;
  prepareCliAction(input: PrepareCliActionInput): Promise<OperationTask>;
  getCliActionPlan(planId: string): Promise<CliActionPlan>;
  executeCliAction(input: ExecuteCliActionInput): Promise<OperationTask>;
  prepareCliBulkUpgrade(agentIds: string[]): Promise<OperationTask>;
  getCliBulkActionPlan(planId: string): Promise<CliBulkActionPlan>;
  executeCliBulkAction(input: ExecuteCliActionInput): Promise<OperationTask>;
  runCliDoctor(agentId: string): Promise<OperationTask>;
}

// Save and reset both carry `expectedRevision` and `catalogVersion`. A caller that has not read a
// profile cannot construct either input, which is the point: a blind write would silently overwrite
// whatever another window saved.
export interface CliParameterService {
  listCliParameterProfiles(): Promise<CliParameterProfile[]>;
  /** Read-only. Renders a draft without touching stored selections or the revision. */
  previewCliParameterProfile(input: PreviewCliParameterProfileInput): Promise<CliParameterPreview>;
  saveCliParameterProfile(input: SaveCliParameterProfileInput): Promise<CliParameterProfile>;
  resetCliParameterProfile(input: ResetCliParameterProfileInput): Promise<CliParameterProfile>;
}

// `applyCliConfigProfile` is deliberately absent: it asserts that switching a profile leaves
// `workflowState` and the active session untouched, so it reads state this context does not own
// and stays implemented in the composition root.
export interface CliConfigService {
  listCliConfigPresets(agentId: string): Promise<CliConfigPreset[]>;
  listCliConfigProfiles(agentId: string): Promise<CliConfigProfile[]>;
  getCliConfigStatus(agentId: string): Promise<CliConfigStatus>;
  saveCliConfigProfile(input: SaveCliConfigProfileInput): Promise<CliConfigProfile>;
  validateCliConfigCredential(input: ValidateCliConfigCredentialInput): Promise<ProviderCredentialValidationResult>;
  duplicateCliConfigProfile(agentId: string, profileId: string): Promise<CliConfigProfile>;
  deleteCliConfigProfile(input: DeleteCliConfigProfileInput): Promise<void>;
  importCliConfigProfile(input: ImportCliConfigProfileInput): Promise<CliConfigProfile>;
  discoverCliConfigProfiles(agentId: string): Promise<CliConfigDiscoveryResult>;
  importDiscoveredCliConfigProfiles(input: ImportDiscoveredCliConfigInput): Promise<ImportDiscoveredCliConfigResult>;
}
