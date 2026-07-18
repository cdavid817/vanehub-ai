import type { PluginIntegrationService } from "./plugin-integration-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriPluginIntegrationClient } from "./tauri-plugin-integration-client";
import { webPluginIntegrationClient } from "./web-plugin-integration-client";

export function createPluginIntegrationService(): PluginIntegrationService {
  return createRuntimeAdapter({
    tauri: tauriPluginIntegrationClient,
    webMock: webPluginIntegrationClient,
  });
}

export const pluginIntegrationService = createPluginIntegrationService();
