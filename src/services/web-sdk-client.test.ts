import { afterEach, describe, expect, it, vi } from "vitest";
import { webOperationClient } from "./web-operation-client";
import { webSdkClient } from "./web-sdk-client";

describe("webSdkClient", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns deterministic SDK definitions and statuses", async () => {
    const definitions = await webSdkClient.listDefinitions();
    const statuses = await webSdkClient.listStatuses();

    expect(definitions.map((definition) => definition.id)).toEqual(["claude-sdk", "codex-sdk"]);
    expect(statuses["claude-sdk"].npmPackage).toBe("@anthropic-ai/claude-agent-sdk");
    expect(statuses["codex-sdk"].status).toBe("not-installed");
  });

  it("returns fallback-ready version information", async () => {
    const versions = await webSdkClient.getVersions("codex-sdk");

    expect(versions["codex-sdk"].versions[0]).toBe("0.117.0");
    expect(versions["codex-sdk"].fallbackVersions).toContain("0.115.0");
  });

  it("simulates install and uninstall operation logs", async () => {
    vi.useFakeTimers();
    const install = await webSdkClient.install({ sdkId: "codex-sdk", version: "0.117.0" });
    expect(install.kind).toBe("sdk");
    expect(install.status).toBe("queued");
    expect(install.logs.length).toBeGreaterThan(0);
    await vi.advanceTimersByTimeAsync(950);
    const completedInstall = await webOperationClient.getOperationStatus(install.id);
    expect(completedInstall.status).toBe("succeeded");

    const statuses = await webSdkClient.listStatuses();
    expect(statuses["codex-sdk"].status).toBe("installed");

    const uninstall = await webSdkClient.uninstall("codex-sdk");
    expect(uninstall.kind).toBe("sdk");
    await vi.advanceTimersByTimeAsync(950);
    const completedUninstall = await webOperationClient.getOperationStatus(uninstall.id);
    expect(completedUninstall.status).toBe("succeeded");

    const logs = await webSdkClient.getOperationLogs("codex-sdk");
    expect(logs.some((entry) => entry.operation === "uninstall")).toBe(true);
  });
});
