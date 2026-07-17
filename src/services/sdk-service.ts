import type {
  SdkDefinition,
  SdkEnvironmentStatus,
  SdkId,
  SdkOperationRequest,
  SdkStatusMap,
  SdkUpdateMap,
  SdkVersionMap,
} from "../types/sdk";
import type { OperationTask } from "../types/operation";

export interface SdkService {
  listDefinitions(): Promise<SdkDefinition[]>;
  listStatuses(): Promise<SdkStatusMap>;
  checkEnvironment(): Promise<SdkEnvironmentStatus>;
  getVersions(sdkId?: SdkId): Promise<SdkVersionMap>;
  checkUpdates(sdkId?: SdkId): Promise<SdkUpdateMap>;
  install(request: SdkOperationRequest): Promise<OperationTask>;
  update(request: SdkOperationRequest): Promise<OperationTask>;
  rollback(request: SdkOperationRequest): Promise<OperationTask>;
  uninstall(sdkId: SdkId): Promise<OperationTask>;
  getOperationLogs(sdkId?: SdkId): Promise<import("../types/sdk").SdkOperationLog[]>;
}
