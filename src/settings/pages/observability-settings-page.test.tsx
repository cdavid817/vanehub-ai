import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../../i18n";
import type { ObservabilitySettings } from "../../types/execution-observability";
import { ObservabilitySettingsPage, validateObservabilitySettings } from "./observability-settings-page";

const settings: ObservabilitySettings = {
  localTimelineEnabled: true,
  otlpEnabled: false,
  otlpEndpoint: null,
  otlpProtocol: "http_protobuf",
  samplingRatio: 1,
  retentionDays: 30,
  capturePolicy: "metadata_only",
  mcpRelayEnabled: false,
  otlpAuthConfigured: false,
};

describe("ObservabilitySettingsPage", () => {
  it("validates bounded retention, sampling, and safe endpoints", () => {
    expect(validateObservabilitySettings(settings)).toEqual({});
    expect(validateObservabilitySettings({ ...settings, retentionDays: 0 })).toHaveProperty("retentionDays");
    expect(validateObservabilitySettings({ ...settings, samplingRatio: 2 })).toHaveProperty("samplingRatio");
    expect(validateObservabilitySettings({ ...settings, otlpEnabled: true, otlpEndpoint: "https://token@example.com/v1/traces" })).toHaveProperty("otlpEndpoint");
  });

  it("renders safe defaults and discloses prospective native export and unavailable relay", () => {
    const client = new QueryClient();
    client.setQueryData(["execution-observability", "settings"], settings);
    client.setQueryData(["execution-observability", "capabilities"], []);
    const service = {
      async getSettings() { return settings; },
      async updateSettings(input: ObservabilitySettings) { return input; },
      async listRuns() { return { items: [], nextPageToken: null }; },
      async getRun() { throw new Error("not used"); },
      async getTimeline() { throw new Error("not used"); },
      async getObservationCapabilities() { return []; },
    };
    const html = renderToString(
      <QueryClientProvider client={client}>
        <ObservabilitySettingsPage service={service} />
      </QueryClientProvider>,
    );
    expect(html).toContain("执行可观测性");
    expect(html).toContain("仅元数据");
    expect(html).toContain("设置对后续运行生效");
    expect(html).toContain("中继激活保持不可用");
  });
});
