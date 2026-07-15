import { invoke } from "@tauri-apps/api/core";
import type { OperationTask } from "../types/operation";
import type { OperationService } from "./operation-service";

export const tauriOperationClient: OperationService = {
  listOperations() {
    return invoke<OperationTask[]>("list_operations");
  },

  getOperationStatus(operationId: string) {
    return invoke<OperationTask>("get_operation_status", { operationId });
  },
};
