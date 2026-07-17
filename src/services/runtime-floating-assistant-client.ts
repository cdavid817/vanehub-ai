import type { FloatingAssistantService } from "./floating-assistant-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriFloatingAssistantClient } from "./tauri-floating-assistant-client";
import { webFloatingAssistantClient } from "./web-floating-assistant-client";

export function createFloatingAssistantService(): FloatingAssistantService {
  return createRuntimeAdapter({
    tauri: tauriFloatingAssistantClient,
    webMock: webFloatingAssistantClient,
  });
}

export const floatingAssistantService = createFloatingAssistantService();
