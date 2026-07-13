import { describe, expect, it } from "vitest";
import { webAgentClient } from "./web-agent-client";

describe("webAgentClient", () => {
  it("lists agents and filters by capability tag", async () => {
    const allAgents = await webAgentClient.listAgents();
    const browserAgents = await webAgentClient.listAgents("browser");

    expect(allAgents.length).toBeGreaterThan(0);
    expect(browserAgents.every((agent) => agent.capabilityTags.includes("browser"))).toBe(true);
  });

  it("selects compatible agents and rejects unsupported interaction modes", async () => {
    await expect(webAgentClient.selectAgent("opencode", "browser")).rejects.toThrow("does not support");

    const workflow = await webAgentClient.selectAgent("gemini-cli", "browser");

    expect(workflow.activeAgentId).toBe("gemini-cli");
    expect(workflow.activeInteractionMode).toBe("browser");
  });

  it("reports browser readiness and session details", async () => {
    const readiness = await webAgentClient.checkBrowserReadiness("gemini-cli");
    const launch = await webAgentClient.launchActiveWorkflow();
    const details = await webAgentClient.getSessionDetails();

    expect(readiness.ready).toBe(true);
    expect(launch.workflow.lifecycleState).toBe("running");
    expect(details.adapter).toBe("browser");
  });
});
