import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getWebImDebugSnapshot, resetWebImMock, webImClient } from "./web-im-client";

describe("web IM client", () => {
  beforeEach(() => resetWebImMock());
  afterEach(() => vi.useRealTimers());

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

  it("enables a configured connector without routing defaults", async () => {
    await webImClient.saveConnector({
      kind: "telegram",
      enabled: false,
      publicConfig: {},
      credentials: { botToken: "write-only-token" },
    });
    await expect(webImClient.setConnectorEnabled("telegram", true)).resolves.toBeUndefined();
  });

  it("creates and cancels session-scoped pairing without project defaults", async () => {
    await webImClient.saveConnector({
      kind: "telegram",
      enabled: true,
      publicConfig: {},
      credentials: { botToken: "write-only-token" },
    });

    const pairing = await webImClient.beginPairing("session-1", "telegram");
    expect(pairing).toMatchObject({ connector: "telegram", sessionId: "session-1" });
    expect(pairing.code).toHaveLength(8);
    await expect(webImClient.getSessionBinding("session-1")).resolves.toEqual({
      access: { connector: "feishu", enabled: false, sessionId: "session-1", updatedAt: "1970-01-01T00:00:00Z" },
      binding: null,
      pendingConnector: "telegram",
    });
    await expect(webImClient.cancelPairing("session-1", "telegram")).resolves.toBe(true);
  });

  it("keeps Feishu access default-off and isolated by session", async () => {
    await expect(webImClient.getSessionBinding("session-a")).resolves.toMatchObject({
      access: { connector: "feishu", enabled: false, sessionId: "session-a" },
    });
    await expect(webImClient.setSessionAccess("session-a", "feishu", true)).resolves.toMatchObject({
      enabled: true,
      sessionId: "session-a",
    });
    await expect(webImClient.getSessionBinding("session-a")).resolves.toMatchObject({
      access: { enabled: true },
    });
    await expect(webImClient.getSessionBinding("session-b")).resolves.toMatchObject({
      access: { enabled: false, sessionId: "session-b" },
    });
  });

  it("requires session access before Feishu pairing and allows re-enable", async () => {
    await webImClient.saveConnector({
      kind: "feishu",
      enabled: true,
      publicConfig: { appId: "fixture-app" },
      credentials: { appSecret: "write-only-secret" },
    });
    await expect(webImClient.beginPairing("session-1", "feishu"))
      .rejects.toThrow("im-session-disabled");
    await webImClient.setSessionAccess("session-1", "feishu", true);
    await expect(webImClient.beginPairing("session-1", "feishu"))
      .resolves.toMatchObject({ connector: "feishu", sessionId: "session-1" });
    await webImClient.setSessionAccess("session-1", "feishu", false);
    await webImClient.setSessionAccess("session-1", "feishu", true);
    await expect(webImClient.getSessionBinding("session-1")).resolves.toMatchObject({
      access: { enabled: true },
    });
  });

  it("preserves a manual binding pause across Feishu access disable and re-enable", async () => {
    await webImClient.saveConnector({
      kind: "feishu",
      enabled: true,
      publicConfig: { appId: "fixture-app" },
      credentials: { appSecret: "write-only-secret" },
    });
    await webImClient.setSessionAccess("session-1", "feishu", true);
    vi.useFakeTimers();
    await webImClient.beginPairing("session-1", "feishu");
    await vi.advanceTimersByTimeAsync(500);
    await webImClient.setBindingPaused("session-1", true);

    await webImClient.setSessionAccess("session-1", "feishu", false);
    await webImClient.setSessionAccess("session-1", "feishu", true);

    await expect(webImClient.getSessionBinding("session-1")).resolves.toMatchObject({
      access: { enabled: true },
      binding: { state: "paused" },
    });
  });

  it("simulates IM-side pairing completion deterministically", async () => {
    await webImClient.saveConnector({
      kind: "telegram",
      enabled: true,
      publicConfig: {},
      credentials: { botToken: "write-only-token" },
    });
    vi.useFakeTimers();
    await webImClient.beginPairing("session-1", "telegram");

    await vi.advanceTimersByTimeAsync(500);

    await expect(webImClient.getSessionBinding("session-1")).resolves.toMatchObject({
      access: { connector: "feishu", enabled: false, sessionId: "session-1" },
      binding: { connector: "telegram", sessionId: "session-1", state: "active" },
      pendingConnector: null,
    });
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
    expect(events).toEqual(["disabled", "connected"]);
  });
});
