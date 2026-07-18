export type PluginIntegrationId = "github";

export type PluginIntegrationStatus =
  | "configured"
  | "not-configured"
  | "missing-cli"
  | "unavailable"
  | "error";

export interface PluginIntegrationSetupStep {
  id: string;
  labelKey: string;
}

export interface PluginIntegrationDefinition {
  id: PluginIntegrationId;
  nameKey: string;
  descriptionKey: string;
  version: string;
  provider: string;
  icon: "github";
  docsUrl: string;
  setupSteps: PluginIntegrationSetupStep[];
}

export interface PluginIntegrationState {
  integrationId: PluginIntegrationId;
  status: PluginIntegrationStatus;
  configured: boolean;
  canTest: boolean;
  lastCheckedAt: string | null;
  statusReasonKey: string | null;
  message: string | null;
}

export interface PluginIntegrationEnvironment {
  runtime: "tauri" | "web-mock";
  nativeChecksAvailable: boolean;
  reasonKey: string | null;
}

export interface PluginIntegrationOverview {
  definitions: PluginIntegrationDefinition[];
  states: PluginIntegrationState[];
  environment: PluginIntegrationEnvironment;
}

export interface PluginIntegrationRequest {
  integrationId: PluginIntegrationId;
}

export interface PluginIntegrationTestResult {
  integrationId: PluginIntegrationId;
  status: PluginIntegrationStatus;
  configured: boolean;
  message: string;
  checkedAt: string;
}
