export type SdkId = "claude-sdk" | "codex-sdk";

export type SdkInstallStatus = "installed" | "not-installed" | "installing" | "uninstalling" | "error";

export type SdkVersionSource = "remote" | "fallback";

export type SdkOperationType = "install" | "update" | "rollback" | "uninstall";

export interface SdkDefinition {
  id: SdkId;
  displayName: string;
  npmPackage: string;
  companionPackages: string[];
  fallbackVersions: string[];
  description: string;
  relatedProviders: string[];
}

export interface SdkStatus {
  id: SdkId;
  displayName: string;
  npmPackage: string;
  description: string;
  relatedProviders: string[];
  status: SdkInstallStatus;
  installedVersion?: string | null;
  latestVersion?: string | null;
  hasUpdate?: boolean;
  installPath?: string | null;
  lastChecked?: string | null;
  errorMessage?: string | null;
}

export interface SdkVersionInfo {
  sdkId: SdkId;
  versions: string[];
  fallbackVersions: string[];
  source: SdkVersionSource;
  latestVersion?: string | null;
  error?: string | null;
}

export interface SdkEnvironmentStatus {
  available: boolean;
  nodePath?: string | null;
  nodeVersion?: string | null;
  npmPath?: string | null;
  npmVersion?: string | null;
  error?: string | null;
}

export interface SdkOperationLog {
  sdkId: SdkId;
  operation: SdkOperationType;
  line: string;
  timestamp: string;
}

export interface SdkOperationRequest {
  sdkId: SdkId;
  version?: string | null;
}

export interface SdkOperationResult {
  success: boolean;
  operationId?: string | null;
  sdkId: SdkId;
  operation: SdkOperationType;
  installedVersion?: string | null;
  requestedVersion?: string | null;
  logs: SdkOperationLog[];
  error?: string | null;
}

export type SdkStatusMap = Record<SdkId, SdkStatus>;
export type SdkVersionMap = Record<SdkId, SdkVersionInfo>;
export type SdkUpdateMap = Record<SdkId, Pick<SdkStatus, "id" | "latestVersion" | "hasUpdate" | "errorMessage">>;

export type SdkVersionAction = "install" | "update" | "rollback" | "current";
