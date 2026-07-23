import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { tauriExecutionObservabilityClient } from "./tauri-execution-observability-client";

describe("Tauri execution observability adapter", () => {
  beforeEach(() => invoke.mockReset().mockResolvedValue({}));

  it("keeps invoke calls confined to the native adapter with contract-shaped arguments", async () => {
    await tauriExecutionObservabilityClient.getSettings();
    expect(invoke).toHaveBeenLastCalledWith("get_observability_settings");

    const settings = {
      localTimelineEnabled: true,
      otlpEnabled: false,
      otlpEndpoint: null,
      otlpProtocol: "http_protobuf" as const,
      samplingRatio: 1,
      retentionDays: 30,
      capturePolicy: "metadata_only" as const,
      mcpRelayEnabled: false,
      otlpAuthConfigured: false,
    };
    await tauriExecutionObservabilityClient.updateSettings(settings);
    expect(invoke).toHaveBeenLastCalledWith("update_observability_settings", { settings });

    await tauriExecutionObservabilityClient.listRuns({
      limit: 25,
      pageToken: "cursor",
      sessionId: "session-1",
    });
    expect(invoke).toHaveBeenLastCalledWith("list_execution_runs", {
      request: { limit: 25, pageToken: "cursor" },
      sessionId: "session-1",
    });

    await tauriExecutionObservabilityClient.getRun("run-1");
    expect(invoke).toHaveBeenLastCalledWith("get_execution_run", { runId: "run-1" });
    await tauriExecutionObservabilityClient.getTimeline("run-1");
    expect(invoke).toHaveBeenLastCalledWith("get_execution_timeline", { runId: "run-1" });
    await tauriExecutionObservabilityClient.getObservationCapabilities();
    expect(invoke).toHaveBeenLastCalledWith("get_execution_observation_capabilities");
  });
});
