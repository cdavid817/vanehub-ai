import type { ExecutionObservabilityService } from "./execution-observability-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriExecutionObservabilityClient } from "./tauri-execution-observability-client";
import { webExecutionObservabilityClient } from "./web-execution-observability-client";

export function createExecutionObservabilityService(): ExecutionObservabilityService {
  return createRuntimeAdapter({
    tauri: tauriExecutionObservabilityClient,
    webMock: webExecutionObservabilityClient,
  });
}

export const executionObservabilityService = createExecutionObservabilityService();
