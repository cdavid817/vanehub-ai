import type {
  CliPackageOperationInput,
  CliParameterProfile,
  CliToolStatus,
  ManagedCliAgentId,
  SaveCliParameterProfileInput,
} from "../types/agent";
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

export interface CliParameterService {
  listCliParameterProfiles(): Promise<CliParameterProfile[]>;
  saveCliParameterProfile(input: SaveCliParameterProfileInput): Promise<CliParameterProfile>;
  resetCliParameterProfile(agentId: ManagedCliAgentId): Promise<CliParameterProfile>;
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
