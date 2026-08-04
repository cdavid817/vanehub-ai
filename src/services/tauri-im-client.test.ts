import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke, listen } = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import { tauriImClient } from "./tauri-im-client";

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
});
