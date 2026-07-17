import type { ExtensionService } from "./extension-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriExtensionClient } from "./tauri-extension-client";
import { webExtensionClient } from "./web-extension-client";

export function createExtensionService(): ExtensionService {
  return createRuntimeAdapter({
    tauri: tauriExtensionClient,
    webMock: webExtensionClient,
  });
}

export const extensionService = createExtensionService();
