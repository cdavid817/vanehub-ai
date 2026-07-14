import { describe, expect, it } from "vitest";
import { webSdkClient } from "./web-sdk-client";

describe("webSdkClient", () => {
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
    const install = await webSdkClient.install({ sdkId: "codex-sdk", version: "0.117.0" });
    expect(install.success).toBe(true);
    expect(install.logs.length).toBeGreaterThan(0);

    const statuses = await webSdkClient.listStatuses();
    expect(statuses["codex-sdk"].status).toBe("installed");

    const uninstall = await webSdkClient.uninstall("codex-sdk");
    expect(uninstall.success).toBe(true);

    const logs = await webSdkClient.getOperationLogs("codex-sdk");
    expect(logs.some((entry) => entry.operation === "uninstall")).toBe(true);
  });
});
