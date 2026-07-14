import type {
  SdkDefinition,
  SdkEnvironmentStatus,
  SdkId,
  SdkOperationRequest,
  SdkOperationResult,
  SdkStatusMap,
  SdkUpdateMap,
  SdkVersionMap,
} from "../types/sdk";

export interface SdkService {
  listDefinitions(): Promise<SdkDefinition[]>;
  listStatuses(): Promise<SdkStatusMap>;
  checkEnvironment(): Promise<SdkEnvironmentStatus>;
  getVersions(sdkId?: SdkId): Promise<SdkVersionMap>;
  checkUpdates(sdkId?: SdkId): Promise<SdkUpdateMap>;
  install(request: SdkOperationRequest): Promise<SdkOperationResult>;
  update(request: SdkOperationRequest): Promise<SdkOperationResult>;
  rollback(request: SdkOperationRequest): Promise<SdkOperationResult>;
  uninstall(sdkId: SdkId): Promise<SdkOperationResult>;
  getOperationLogs(sdkId?: SdkId): Promise<SdkOperationResult["logs"]>;
}
