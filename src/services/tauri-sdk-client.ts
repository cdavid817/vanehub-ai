import { invoke } from "@tauri-apps/api/core";
import type { SdkService } from "./sdk-service";
import type {
  SdkDefinition,
  SdkEnvironmentStatus,
  SdkId,
  SdkOperationLog,
  SdkOperationRequest,
  SdkOperationResult,
  SdkStatusMap,
  SdkUpdateMap,
  SdkVersionMap,
} from "../types/sdk";

export const tauriSdkClient: SdkService = {
  listDefinitions() {
    return invoke<SdkDefinition[]>("list_sdk_definitions");
  },

  listStatuses() {
    return invoke<SdkStatusMap>("list_sdk_statuses");
  },

  checkEnvironment() {
    return invoke<SdkEnvironmentStatus>("check_sdk_environment");
  },

  getVersions(sdkId?: SdkId) {
    return invoke<SdkVersionMap>("get_sdk_versions", { sdkId: sdkId ?? null });
  },

  checkUpdates(sdkId?: SdkId) {
    return invoke<SdkUpdateMap>("check_sdk_updates", { sdkId: sdkId ?? null });
  },

  install(request: SdkOperationRequest) {
    return invoke<SdkOperationResult>("install_sdk_dependency", { request });
  },

  update(request: SdkOperationRequest) {
    return invoke<SdkOperationResult>("update_sdk_dependency", { request });
  },

  rollback(request: SdkOperationRequest) {
    return invoke<SdkOperationResult>("rollback_sdk_dependency", { request });
  },

  uninstall(sdkId: SdkId) {
    return invoke<SdkOperationResult>("uninstall_sdk_dependency", { sdkId });
  },

  getOperationLogs(sdkId?: SdkId) {
    return invoke<SdkOperationLog[]>("get_sdk_operation_logs", { sdkId: sdkId ?? null });
  },
};
