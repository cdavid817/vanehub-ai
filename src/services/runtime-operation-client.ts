import type { OperationService } from "./operation-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriOperationClient } from "./tauri-operation-client";
import { webOperationClient } from "./web-operation-client";

export function createOperationService(): OperationService {
  return createRuntimeAdapter({
    tauri: tauriOperationClient,
    webMock: webOperationClient,
  });
}

export const operationService = createOperationService();
