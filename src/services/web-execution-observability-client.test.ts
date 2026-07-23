import { beforeEach, describe, expect, it } from "vitest";
import {
  resetWebExecutionObservabilityForTest,
  webExecutionObservabilityClient,
} from "./web-execution-observability-client";

describe("Web execution observability adapter", () => {
  beforeEach(() => resetWebExecutionObservabilityForTest());

  it("uses safe deterministic defaults and never claims native effects", async () => {
    const settings = await webExecutionObservabilityClient.getSettings();
    expect(settings).toMatchObject({
      localTimelineEnabled: true,
      otlpEnabled: false,
      retentionDays: 30,
      capturePolicy: "metadata_only",
      mcpRelayEnabled: false,
      otlpAuthConfigured: false,
    });
    await expect(
      webExecutionObservabilityClient.updateSettings({ ...settings, otlpEnabled: true }),
    ).rejects.toThrow("unavailable in Web preview");
  });

  it("paginates deterministic run summaries and filters by session", async () => {
    const first = await webExecutionObservabilityClient.listRuns({ limit: 1 });
    expect(first.items).toHaveLength(1);
    expect(first.nextPageToken).toBe("web:1");

    const second = await webExecutionObservabilityClient.listRuns({
      limit: 1,
      pageToken: first.nextPageToken,
    });
    expect(second.items[0]?.runId).not.toBe(first.items[0]?.runId);
    const filtered = await webExecutionObservabilityClient.listRuns({
      limit: 10,
      sessionId: "web-session-1",
    });
    expect(filtered.items).toHaveLength(10);
    expect(filtered.nextPageToken).toBe("web:10");
  });

  it("preserves incomplete inferred tool observations and opaque MCP capability", async () => {
    const page = await webExecutionObservabilityClient.listRuns({ limit: 1 });
    const timeline = await webExecutionObservabilityClient.getTimeline(page.items[0]!.runId);
    expect(timeline.spans.find((span) => span.name.startsWith("execute_tool"))).toMatchObject({
      status: "incomplete",
      fidelity: "inferred",
      durationMs: null,
    });
    expect(timeline.spans.find((span) => span.name.startsWith("mcp."))).toMatchObject({
      status: "incomplete",
      fidelity: "opaque",
      durationMs: null,
    });
    const capabilities = await webExecutionObservabilityClient.getObservationCapabilities();
    expect(capabilities).toHaveLength(8);
    expect(capabilities.every((capability) => capability.mcpFidelity === "opaque")).toBe(true);
    expect(capabilities.every((capability) => !capability.relaySupported)).toBe(true);
  });
});
