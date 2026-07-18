import type {
  PluginIntegrationOverview,
  PluginIntegrationRequest,
  PluginIntegrationTestResult,
} from "../types/plugin-integration";

export interface PluginIntegrationService {
  getOverview(): Promise<PluginIntegrationOverview>;
  refresh(): Promise<PluginIntegrationOverview>;
  testReadiness(request: PluginIntegrationRequest): Promise<PluginIntegrationTestResult>;
}
