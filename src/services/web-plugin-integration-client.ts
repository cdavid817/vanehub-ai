import type { PluginIntegrationService } from "./plugin-integration-service";
import type {
  PluginIntegrationDefinition,
  PluginIntegrationOverview,
  PluginIntegrationState,
  PluginIntegrationTestResult,
} from "../types/plugin-integration";

const githubDefinition: PluginIntegrationDefinition = {
  id: "github",
  nameKey: "plugins.github.name",
  descriptionKey: "plugins.github.description",
  version: "1.0.0",
  provider: "GitHub",
  icon: "github",
  docsUrl: "https://cli.github.com/manual/gh_auth_login",
  setupSteps: [
    { id: "install", labelKey: "plugins.github.setup.install" },
    { id: "auth", labelKey: "plugins.github.setup.auth" },
  ],
};

let githubState: PluginIntegrationState = {
  integrationId: "github",
  status: "unavailable",
  configured: false,
  canTest: false,
  lastCheckedAt: null,
  statusReasonKey: "plugins.environment.desktopOnly",
  message: null,
};

function overview(): PluginIntegrationOverview {
  return {
    definitions: [githubDefinition],
    states: [githubState],
    environment: {
      runtime: "web-mock",
      nativeChecksAvailable: false,
      reasonKey: "plugins.environment.desktopOnly",
    },
  };
}

export const webPluginIntegrationClient: PluginIntegrationService = {
  async getOverview() {
    return overview();
  },
  async refresh() {
    return overview();
  },
  async testReadiness({ integrationId }): Promise<PluginIntegrationTestResult> {
    if (integrationId !== "github") {
      throw new Error(`Unknown plugin integration: ${integrationId}`);
    }
    const checkedAt = new Date().toISOString();
    githubState = { ...githubState, lastCheckedAt: checkedAt };
    return {
      integrationId,
      status: "unavailable",
      configured: false,
      message: "plugins.environment.desktopOnly",
      checkedAt,
    };
  },
};

export function resetWebPluginIntegrationStateForTests() {
  githubState = {
    integrationId: "github",
    status: "unavailable",
    configured: false,
    canTest: false,
    lastCheckedAt: null,
    statusReasonKey: "plugins.environment.desktopOnly",
    message: null,
  };
}
