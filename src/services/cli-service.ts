import type { CliPackageOperationInput, CliToolStatus } from "../types/agent";
import type {
  CliParameterPreview,
  CliParameterProfile,
  PreviewCliParameterProfileInput,
  ResetCliParameterProfileInput,
  SaveCliParameterProfileInput,
} from "../types/cli-parameter-profile";
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

export interface CliToolService {
  listCliTools(): Promise<CliToolStatus[]>;
  refreshCliDetections(agentId?: string): Promise<OperationTask>;
  installCliVersion(input: CliPackageOperationInput): Promise<OperationTask>;
  upgradeAllCliVersions(): Promise<OperationTask>;
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
