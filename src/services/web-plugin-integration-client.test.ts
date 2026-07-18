import { describe, expect, it } from "vitest";
import { resetWebPluginIntegrationStateForTests, webPluginIntegrationClient } from "./web-plugin-integration-client";

describe("webPluginIntegrationClient", () => {
  it("returns the stable built-in GitHub integration without claiming native checks", async () => {
    resetWebPluginIntegrationStateForTests();
    const overview = await webPluginIntegrationClient.getOverview();

    expect(overview.definitions.map((definition) => definition.id)).toEqual(["github"]);
    expect(overview.environment.nativeChecksAvailable).toBe(false);
    expect(overview.states[0]).toMatchObject({
      integrationId: "github",
      status: "unavailable",
      configured: false,
      canTest: false,
    });
  });

  it("returns a desktop-only readiness result in Web mock mode", async () => {
    resetWebPluginIntegrationStateForTests();
    const result = await webPluginIntegrationClient.testReadiness({ integrationId: "github" });

    expect(result).toMatchObject({
      integrationId: "github",
      status: "unavailable",
      configured: false,
      message: "plugins.environment.desktopOnly",
    });
  });
});
