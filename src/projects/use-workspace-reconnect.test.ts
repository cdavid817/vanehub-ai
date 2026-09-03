// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SshConnectionTestResult } from "../types/ssh-connection";

const mocks = vi.hoisted(() => ({ testConnection: vi.fn() }));

vi.mock("../services/runtime-ssh-connection-client", () => ({
  sshConnectionService: { testConnection: mocks.testConnection },
}));

import { useWorkspaceReconnect } from "./use-workspace-reconnect";

const succeeded: SshConnectionTestResult = { status: "succeeded", message: "ok", testedAt: "2026-08-20T00:00:00.000Z" };

describe("useWorkspaceReconnect", () => {
  beforeEach(() => {
    mocks.testConnection.mockReset();
  });

  it("calls SshConnectionService.testConnection with the matched connection id, keyed by workspaceId", async () => {
    mocks.testConnection.mockResolvedValue(succeeded);
    const onReconnected = vi.fn();
    const { result } = renderHook(() => useWorkspaceReconnect(onReconnected));

    await act(async () => {
      await result.current.reconnect("ssh://vane@dev.example.com/work/app", "conn-1");
    });

    expect(mocks.testConnection).toHaveBeenCalledWith("conn-1");
    expect(mocks.testConnection).toHaveBeenCalledTimes(1);
  });

  it("shows pending state while the test is in flight, keyed by the workspace id", () => {
    mocks.testConnection.mockReturnValue(new Promise(() => undefined));
    const { result } = renderHook(() => useWorkspaceReconnect(vi.fn()));

    act(() => { void result.current.reconnect("workspace-1", "conn-1"); });

    expect(result.current.mutations.get("workspace-1")?.pending).toBe(true);
  });

  it("calls onReconnected only after a successful test", async () => {
    mocks.testConnection.mockResolvedValue(succeeded);
    const onReconnected = vi.fn();
    const { result } = renderHook(() => useWorkspaceReconnect(onReconnected));

    await act(async () => {
      await result.current.reconnect("workspace-1", "conn-1");
    });

    expect(onReconnected).toHaveBeenCalledTimes(1);
    expect(result.current.mutations.get("workspace-1")).toBeUndefined();
  });

  it("records a non-retryable error and never calls onReconnected when the test rejects", async () => {
    mocks.testConnection.mockRejectedValue(new Error("Connection refused"));
    const onReconnected = vi.fn();
    const { result } = renderHook(() => useWorkspaceReconnect(onReconnected));

    await act(async () => {
      await result.current.reconnect("workspace-1", "conn-1");
    });

    expect(onReconnected).not.toHaveBeenCalled();
    expect(result.current.mutations.get("workspace-1")).toEqual({
      targetKey: "workspace-1",
      operationId: undefined,
      pending: false,
      error: { kind: "error", message: "Connection refused", retryable: false },
    });
  });
});
