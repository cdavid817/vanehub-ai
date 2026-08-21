import type { ExtensionPlatformService } from "./extension-platform-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriExtensionPlatformClient } from "./tauri-extension-platform-client";
import { webExtensionPlatformClient } from "./web-extension-platform-client";

export function createExtensionPlatformService(): ExtensionPlatformService {
  return createRuntimeAdapter({
    tauri: tauriExtensionPlatformClient,
    webMock: webExtensionPlatformClient,
  });
}

export const extensionPlatformService = createExtensionPlatformService();
