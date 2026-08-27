import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke, listen } = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import { tauriImClient } from "./tauri-im-client";
import { webImClient } from "./web-im-client";

describe("Tauri IM client contract", () => {
  beforeEach(() => {
    invoke.mockReset();
    listen.mockReset();
  });

  it("returns the normalized routing produced by native persistence", async () => {
    invoke.mockResolvedValue({ agentId: "codex-cli", projectPath: "D:\\normalized" });

    await expect(tauriImClient.saveRouting({ agentId: " codex-cli ", projectPath: " D:\\normalized " }))
      .resolves.toEqual({ agentId: "codex-cli", projectPath: "D:\\normalized" });
    expect(invoke).toHaveBeenCalledWith("save_im_routing", {
      routing: { agentId: " codex-cli ", projectPath: " D:\\normalized " },
    });
  });

  it("passes edited fields as a patch and returns the normalized connector result", async () => {
    const input = {
      kind: "feishu" as const,
      enabled: false,
      publicConfig: { appId: "persisted-id" },
      credentials: { appSecret: "replacement-secret" },
    };
    invoke.mockResolvedValue({
      kind: "feishu",
      enabled: false,
      displayName: null,
      publicConfig: { appId: "persisted-id" },
      credentialRef: "im://feishu/credentials",
    });

    await expect(tauriImClient.saveConnector(input)).resolves.toMatchObject({
      publicConfig: { appId: "persisted-id" },
      credentialRef: "im://feishu/credentials",
    });
    expect(invoke).toHaveBeenCalledWith("save_im_connector", { input });
  });

  it("validates lifecycle events and returns the native unsubscribe callback", async () => {
    const unsubscribe = vi.fn();
    let listener: ((event: { payload: unknown }) => void) | undefined;
    listen.mockImplementation(async (_event, handler) => {
      listener = handler;
      return unsubscribe;
    });
    const handler = vi.fn();

    await expect(tauriImClient.subscribeLifecycle(handler)).resolves.toBe(unsubscribe);
    listener?.({ payload: { kind: "telegram", lifecycle: "connected", generation: 2, updatedAt: "2026-01-01" } });
    listener?.({ payload: { kind: "unknown", lifecycle: "connected", generation: -1, updatedAt: 1 } });
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it("keeps Tauri and Web adapters method-for-method compatible", () => {
    expect(Object.keys(tauriImClient).sort()).toEqual(Object.keys(webImClient).sort());
  });

  it("maps session pairing and binding commands through validated contracts", async () => {
    invoke
      .mockResolvedValueOnce({
        connector: "telegram",
        sessionId: "session-1",
        code: "ABCD2345",
        expiresAt: "2026-08-12T10:10:00Z",
        replaceExisting: false,
      })
      .mockResolvedValueOnce({
        access: { connector: "feishu", enabled: false, sessionId: "session-1", updatedAt: "1970-01-01T00:00:00Z" },
        binding: null,
        pendingConnector: "telegram",
      });

    await expect(tauriImClient.beginPairing("session-1", "telegram")).resolves.toMatchObject({
      code: "ABCD2345",
    });
    expect(invoke).toHaveBeenNthCalledWith(1, "begin_im_pairing", {
      connector: "telegram",
      replaceExisting: false,
      sessionId: "session-1",
    });
    await expect(tauriImClient.getSessionBinding("session-1")).resolves.toEqual({
      access: { connector: "feishu", enabled: false, sessionId: "session-1", updatedAt: "1970-01-01T00:00:00Z" },
      binding: null,
      pendingConnector: "telegram",
    });
  });

  it("maps session access mutations and rejects malformed native access responses", async () => {
    const access = {
      connector: "feishu",
      enabled: true,
      sessionId: "session-1",
      updatedAt: "2026-08-12T10:00:00Z",
    };
    invoke
      .mockResolvedValueOnce(access)
      .mockResolvedValueOnce({ ...access, enabled: "true" })
      .mockResolvedValueOnce({ ...access, deliveryCredentialRef: "private-ref" });

    await expect(tauriImClient.setSessionAccess("session-1", "feishu", true)).resolves.toEqual(access);
    expect(invoke).toHaveBeenNthCalledWith(1, "set_im_session_access", {
      connector: "feishu",
      enabled: true,
      sessionId: "session-1",
    });
    await expect(tauriImClient.setSessionAccess("session-1", "feishu", true)).rejects.toThrow();
    await expect(tauriImClient.setSessionAccess("session-1", "feishu", true)).rejects.toThrow();
  });

  it("rejects a malformed access object nested in a native binding snapshot", async () => {
    invoke.mockResolvedValue({
      access: {
        connector: "feishu",
        enabled: false,
        sessionId: "session-1",
      },
      binding: null,
      pendingConnector: null,
    });

    await expect(tauriImClient.getSessionBinding("session-1")).rejects.toThrow();
  });
});
