import { beforeEach, describe, expect, it } from "vitest";
import { getWebImDebugSnapshot, resetWebImMock, webImClient } from "./web-im-client";

describe("web IM client", () => {
  beforeEach(() => resetWebImMock());

  it("never persists submitted credential plaintext", async () => {
    await webImClient.saveRouting({ agentId: "codex-cli", projectPath: "D:\\example" });
    await webImClient.saveConnector({
      kind: "telegram",
      enabled: true,
      publicConfig: {},
      credentials: { botToken: "sentinel-private-value" },
    });

    expect(getWebImDebugSnapshot()).not.toContain("sentinel-private-value");
    expect((await webImClient.listConnectors()).find((item) => item.descriptor.kind === "telegram")?.hasCredentials).toBe(true);
  });

  it("normalizes routing and returns the stored mutation result", async () => {
    await expect(webImClient.saveRouting({ agentId: "  codex-cli ", projectPath: " D:\\example  " }))
      .resolves.toEqual({ agentId: "codex-cli", projectPath: "D:\\example" });
    await expect(webImClient.getRouting()).resolves.toEqual({ agentId: "codex-cli", projectPath: "D:\\example" });
  });

  it("merges partial field patches without dropping omitted configured fields", async () => {
    await webImClient.saveConnector({
      kind: "feishu",
      enabled: false,
      publicConfig: {},
      credentials: { appId: "first-id", appSecret: "write-only-secret" },
    });

    const updated = await webImClient.saveConnector({
      kind: "feishu",
      enabled: false,
      publicConfig: { appId: "first-id" },
      credentials: { appId: "replacement-id" },
    });

    expect(updated.publicConfig).toEqual({ appId: "replacement-id" });
    expect(updated.credentialRef).toBe("mock://feishu/credential");
    expect(getWebImDebugSnapshot()).not.toContain("write-only-secret");
  });

  it("rejects invalid merged patches without changing configured fields", async () => {
    await expect(webImClient.saveConnector({
      kind: "feishu",
      enabled: false,
      publicConfig: {},
      credentials: { appSecret: "secret-without-public-id" },
    })).rejects.toThrow("connector-credentials-incomplete");

    expect((await webImClient.listConnectors()).find((item) => item.descriptor.kind === "feishu"))
      .toMatchObject({ config: { publicConfig: {} }, hasCredentials: false });
  });

  it("runs a deterministic QR lifecycle", async () => {
    const waiting = await webImClient.beginWeChatAuthorization();
    expect(waiting.status).toBe("waiting");
    expect(waiting.imageDataUrl).toMatch(/^data:image\/svg\+xml,/);
    expect((await webImClient.pollWeChatAuthorization()).status).toBe("scanned");
    expect((await webImClient.pollWeChatAuthorization()).status).toBe("confirmed");
  });

  it("models routing and connector state transitions independently", async () => {
    await expect(webImClient.saveRouting({ agentId: "codex-cli", projectPath: "D:\\example" }))
      .resolves.toEqual({ agentId: "codex-cli", projectPath: "D:\\example" });
    await webImClient.saveConnector({
      kind: "telegram",
      enabled: false,
      publicConfig: {},
      credentials: { botToken: "write-only-token" },
    });
    await webImClient.setConnectorEnabled("telegram", true);
    expect((await webImClient.listConnectors()).find((item) => item.descriptor.kind === "telegram"))
      .toMatchObject({ config: { enabled: true }, health: { lifecycle: "connected" }, hasCredentials: true });

    await webImClient.setConnectorEnabled("telegram", false);
    await webImClient.clearConnector("telegram");
    expect((await webImClient.listConnectors()).find((item) => item.descriptor.kind === "telegram"))
      .toMatchObject({ config: { enabled: false, credentialRef: null }, health: { lifecycle: "unconfigured" }, hasCredentials: false });
    expect(getWebImDebugSnapshot()).not.toContain("write-only-token");
  });

  it("rejects native-equivalent enablement without routing defaults", async () => {
    await webImClient.saveConnector({
      kind: "telegram",
      enabled: false,
      publicConfig: {},
      credentials: { botToken: "write-only-token" },
    });
    await expect(webImClient.setConnectorEnabled("telegram", true)).rejects.toThrow("im-routing-required");
  });

  it("cancels QR polling without retaining authorization state", async () => {
    await webImClient.beginWeChatAuthorization();
    await webImClient.cancelWeChatAuthorization();
    await expect(webImClient.pollWeChatAuthorization()).rejects.toThrow("wechat-authorization-not-started");
  });

  it("publishes deterministic lifecycle updates and unsubscribes", async () => {
    const events: string[] = [];
    const unsubscribe = await webImClient.subscribeLifecycle((health) => events.push(health.lifecycle));
    await webImClient.saveConnector({
      kind: "telegram", enabled: false, publicConfig: {}, credentials: { botToken: "token" },
    });
    await webImClient.setConnectorEnabled("telegram", true).catch(() => undefined);
    unsubscribe();
    await webImClient.clearConnector("telegram");
    expect(events).toEqual(["disabled"]);
  });
});
