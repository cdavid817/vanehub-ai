// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderToString } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
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

  it("shows a persistent restart-pending notice only after saving a changed OTLP export field (task 12.15)", async () => {
    const client = new QueryClient();
    client.setQueryData(["execution-observability", "settings"], settings);
    client.setQueryData(["execution-observability", "capabilities"], []);
    const user = userEvent.setup();
    const service = {
      async getSettings() { return settings; },
      async updateSettings(input: ObservabilitySettings) { return input; },
      async listRuns() { return { items: [], nextPageToken: null }; },
      async getRun() { throw new Error("not used"); },
      async getTimeline() { throw new Error("not used"); },
      async getObservationCapabilities() { return []; },
    };
    render(
      <QueryClientProvider client={client}>
        <ObservabilitySettingsPage service={service} />
      </QueryClientProvider>,
    );

    expect(screen.queryByText(/重启 VaneHub 以重建/)).toBeNull();

    // Enabling OTLP export without a valid endpoint fails validation and keeps Save disabled, so
    // the endpoint must be filled in too -- otherwise this test would pass for the wrong reason
    // (Save never actually firing) rather than proving the restart notice.
    await user.click(await screen.findByRole("checkbox", { name: "启用 OTLP/HTTP protobuf 导出" }));
    await user.type(screen.getByRole("textbox", { name: "Collector 地址" }), "https://otel.example.com/v1/traces");
    await user.click(screen.getByRole("button", { name: "保存可观测性设置" }));

    expect(await screen.findByText(/重启 VaneHub 以重建/)).toBeTruthy();
  });

  it("does not show the restart-pending notice after saving an unrelated field (task 12.15)", async () => {
    const client = new QueryClient();
    client.setQueryData(["execution-observability", "settings"], settings);
    client.setQueryData(["execution-observability", "capabilities"], []);
    const user = userEvent.setup();
    const service = {
      async getSettings() { return settings; },
      async updateSettings(input: ObservabilitySettings) { return input; },
      async listRuns() { return { items: [], nextPageToken: null }; },
      async getRun() { throw new Error("not used"); },
      async getTimeline() { throw new Error("not used"); },
      async getObservationCapabilities() { return []; },
    };
    render(
      <QueryClientProvider client={client}>
        <ObservabilitySettingsPage service={service} />
      </QueryClientProvider>,
    );

    await user.click(await screen.findByRole("checkbox", { name: "保存本地执行时间线" }));
    await user.click(screen.getByRole("button", { name: "保存可观测性设置" }));

    expect(await screen.findByText(/可观测性配置已保存/)).toBeTruthy();
    expect(screen.queryByText(/重启 VaneHub 以重建/)).toBeNull();
  });

  it("reports a restart-required status for its nav entry once the same condition is true (task 12.16)", async () => {
    const client = new QueryClient();
    client.setQueryData(["execution-observability", "settings"], settings);
    client.setQueryData(["execution-observability", "capabilities"], []);
    const user = userEvent.setup();
    const onStatusChange = vi.fn();
    const service = {
      async getSettings() { return settings; },
      async updateSettings(input: ObservabilitySettings) { return input; },
      async listRuns() { return { items: [], nextPageToken: null }; },
      async getRun() { throw new Error("not used"); },
      async getTimeline() { throw new Error("not used"); },
      async getObservationCapabilities() { return []; },
    };
    render(
      <QueryClientProvider client={client}>
        <ObservabilitySettingsPage onStatusChange={onStatusChange} service={service} />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith(null));
    await user.click(await screen.findByRole("checkbox", { name: "启用 OTLP/HTTP protobuf 导出" }));
    await user.type(screen.getByRole("textbox", { name: "Collector 地址" }), "https://otel.example.com/v1/traces");
    await user.click(screen.getByRole("button", { name: "保存可观测性设置" }));

    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith({
      kind: "restart-required",
      labelKey: "observability.restartPending",
    }));
  });
});
