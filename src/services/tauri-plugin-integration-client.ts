import { invoke } from "@tauri-apps/api/core";
import type { PluginIntegrationService } from "./plugin-integration-service";
import type { PluginIntegrationOverview, PluginIntegrationTestResult } from "../types/plugin-integration";

export const tauriPluginIntegrationClient: PluginIntegrationService = {
  getOverview() {
    return invoke<PluginIntegrationOverview>("get_plugin_integration_overview");
  },
  refresh() {
    return invoke<PluginIntegrationOverview>("refresh_plugin_integrations");
  },
  testReadiness(request) {
    return invoke<PluginIntegrationTestResult>("test_plugin_integration", { request });
  },
};
