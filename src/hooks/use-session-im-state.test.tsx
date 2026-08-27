// @vitest-environment jsdom

import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type {
  ImConnectorHealth,
  ImConnectorView,
  ImSessionAccess,
  ImSessionBinding,
} from "../contracts/im";
import type { ImService } from "../services/im-service";
import { useSessionImState } from "./use-session-im-state";

const connectedTelegram: ImConnectorView = {
  descriptor: { kind: "telegram", supportsQrAuthorization: false, experimental: false, maxOutboundChars: 4096 },
  config: { kind: "telegram", enabled: true, publicConfig: {}, credentialRef: null },
  health: { kind: "telegram", lifecycle: "connected", generation: 1, updatedAt: "2026-08-13T00:00:00Z" },
  hasCredentials: true,
};

const binding: ImSessionBinding = {
  connector: "telegram",
  sessionId: "session-1",
  state: "active",
  completionNotifications: false,
  createdAt: "2026-08-13T00:00:00Z",
  updatedAt: "2026-08-13T00:00:00Z",
};

function mockService(initialBinding: ImSessionBinding | null = null) {
  let currentBinding = initialBinding;
  const accessBySession = new Map<string, ImSessionAccess>();
  let lifecycleHandler: ((health: ImConnectorHealth) => void) | null = null;
  const unsubscribe = vi.fn();
  const service: ImService = {
    listConnectors: vi.fn(async () => [connectedTelegram]),
    getRouting: vi.fn(async () => null),
    saveRouting: vi.fn(async (routing) => routing),
    saveConnector: vi.fn(),
    setConnectorEnabled: vi.fn(),
    restartConnector: vi.fn(),
    testConnector: vi.fn(),
    clearConnector: vi.fn(),
    resetBindings: vi.fn(),
    getSessionBinding: vi.fn(async (sessionId) => ({
      access: accessBySession.get(sessionId) ?? {
        sessionId,
        connector: "feishu",
        enabled: false,
        updatedAt: "1970-01-01T00:00:00Z",
      },
      binding: currentBinding,
      pendingConnector: null,
    })),
    setSessionAccess: vi.fn(async (sessionId, connector, enabled) => {
      const access = { sessionId, connector, enabled, updatedAt: "2026-08-13T00:01:00Z" };
      accessBySession.set(sessionId, access);
      return access;
    }),
    beginPairing: vi.fn(async (sessionId, connector, replaceExisting = false) => ({
      code: "ABCDEFGH",
      connector,
      expiresAt: new Date(Date.now() + 1_000).toISOString(),
      replaceExisting,
      sessionId,
    })),
    cancelPairing: vi.fn(async () => true),
    setBindingPaused: vi.fn(async (_sessionId, paused) => {
      currentBinding = { ...binding, state: paused ? "paused" : "active" };
      return currentBinding;
    }),
    setCompletionNotifications: vi.fn(async (_sessionId, enabled) => {
      currentBinding = { ...binding, completionNotifications: enabled };
      return currentBinding;
    }),
    removeSessionBinding: vi.fn(async () => {
      currentBinding = null;
      return true;
    }),
    subscribeLifecycle: vi.fn(async (handler) => {
      lifecycleHandler = handler;
      return unsubscribe;
    }),
    beginWeChatAuthorization: vi.fn(),
    pollWeChatAuthorization: vi.fn(),
    cancelWeChatAuthorization: vi.fn(),
  };
  return { emitHealth: (health: ImConnectorHealth) => lifecycleHandler?.(health), service, unsubscribe };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

describe("useSessionImState", () => {
  afterEach(() => vi.useRealTimers());

  it("loads an unbound session and reacts to connector health", async () => {
    const mock = mockService();
    const { result } = renderHook(() => useSessionImState("session-1", mock.service));

    await waitFor(() => expect(result.current.readyConnectors).toHaveLength(1));
    act(() => mock.emitHealth({ ...connectedTelegram.health, lifecycle: "error", generation: 2 }));

    expect(result.current.readyConnectors).toHaveLength(0);
    expect(result.current.connectors[0].health.lifecycle).toBe("error");
  });

  it("loads access default-off and recovers after a failed enable mutation", async () => {
    const mock = mockService();
    vi.mocked(mock.service.setSessionAccess).mockRejectedValueOnce(new Error("native-access-failed"));
    const { result } = renderHook(() => useSessionImState("session-1", mock.service));

    await waitFor(() => expect(result.current.access?.enabled).toBe(false));
    await act(() => result.current.setAccess(true));
    expect(result.current.access?.enabled).toBe(false);
    expect(result.current.error).toBe("native-access-failed");

    await act(() => result.current.setAccess(true));
    expect(result.current.access?.enabled).toBe(true);
    expect(result.current.error).toBeNull();
    await act(() => result.current.setAccess(false));
    await act(() => result.current.setAccess(true));
    expect(result.current.access?.enabled).toBe(true);
  });

  it("discards an access mutation that resolves after the selected session changes", async () => {
    const mock = mockService();
    const stale = deferred<ImSessionAccess>();
    vi.mocked(mock.service.setSessionAccess).mockReturnValueOnce(stale.promise);
    const { result, rerender } = renderHook(
      ({ sessionId }) => useSessionImState(sessionId, mock.service),
      { initialProps: { sessionId: "session-1" as string } },
    );
    await waitFor(() => expect(result.current.access?.sessionId).toBe("session-1"));

    let mutation!: Promise<ImSessionAccess | null>;
    act(() => {
      mutation = result.current.setAccess(true);
    });
    expect(result.current.pending).toBe(true);
    rerender({ sessionId: "session-2" });
    await waitFor(() => {
      expect(result.current.access).toMatchObject({ sessionId: "session-2", enabled: false });
      expect(result.current.pending).toBe(false);
    });

    await act(async () => {
      stale.resolve({
        sessionId: "session-1",
        connector: "feishu",
        enabled: true,
        updatedAt: "2026-08-13T00:02:00Z",
      });
      await mutation;
    });
    expect(result.current.access).toMatchObject({ sessionId: "session-2", enabled: false });
  });

  it("keeps an access mutation when only the same-session reload sequence changes", async () => {
    const mock = mockService();
    const mutationResult = deferred<ImSessionAccess>();
    vi.mocked(mock.service.setSessionAccess).mockReturnValueOnce(mutationResult.promise);
    const { result } = renderHook(() => useSessionImState("session-1", mock.service));
    await waitFor(() => expect(result.current.access?.sessionId).toBe("session-1"));

    let mutation!: Promise<ImSessionAccess | null>;
    act(() => {
      mutation = result.current.setAccess(true);
    });
    await act(() => result.current.reload());
    await act(async () => {
      mutationResult.resolve({
        sessionId: "session-1",
        connector: "feishu",
        enabled: true,
        updatedAt: "2026-08-13T00:03:00Z",
      });
      await mutation;
    });

    expect(result.current.access?.enabled).toBe(true);
    expect(result.current.pending).toBe(false);
  });

  it("clears and cancels plaintext pairing state when the session changes", async () => {
    const mock = mockService();
    const { result, rerender } = renderHook(
      ({ sessionId }) => useSessionImState(sessionId, mock.service),
      { initialProps: { sessionId: "session-1" as string | null } },
    );
    await waitFor(() => expect(result.current.readyConnectors).toHaveLength(1));
    await act(() => result.current.beginPairing("telegram", false));
    expect(result.current.pairing?.code).toBe("ABCDEFGH");

    rerender({ sessionId: "session-2" });

    await waitFor(() => expect(result.current.pairing).toBeNull());
    expect(mock.service.cancelPairing).toHaveBeenCalledWith("session-1", "telegram");
  });

  it("cancels the active code before generating a replacement", async () => {
    const mock = mockService();
    vi.mocked(mock.service.beginPairing)
      .mockResolvedValueOnce({
        code: "ABCDEFGH",
        connector: "telegram",
        expiresAt: new Date(Date.now() + 1_000).toISOString(),
        replaceExisting: true,
        sessionId: "session-1",
      })
      .mockResolvedValueOnce({
        code: "IJKLMNOP",
        connector: "telegram",
        expiresAt: new Date(Date.now() + 1_000).toISOString(),
        replaceExisting: true,
        sessionId: "session-1",
      });
    const { result } = renderHook(() => useSessionImState("session-1", mock.service));
    await waitFor(() => expect(result.current.readyConnectors).toHaveLength(1));
    await act(() => result.current.beginPairing("telegram", true));

    await act(() => result.current.retryPairing());

    expect(mock.service.cancelPairing).toHaveBeenCalledWith("session-1", "telegram");
    expect(mock.service.beginPairing).toHaveBeenLastCalledWith("session-1", "telegram", true);
    expect(result.current.pairing?.code).toBe("IJKLMNOP");
  });

  it("expires pairing locally and removes the plaintext code", async () => {
    vi.useFakeTimers();
    const mock = mockService();
    const { result } = renderHook(() => useSessionImState("session-1", mock.service));
    await act(async () => { await Promise.resolve(); });
    await act(() => result.current.beginPairing("telegram", true));

    await act(async () => { vi.advanceTimersByTime(1_001); });

    expect(result.current.pairing).toBeNull();
    expect(result.current.error).toBe("im-pairing-expired");
    expect(mock.service.cancelPairing).toHaveBeenCalledWith("session-1", "telegram");
  });

  it("uses normalized binding results for pause, notification, and removal", async () => {
    const mock = mockService(binding);
    const { result } = renderHook(() => useSessionImState("session-1", mock.service));
    await waitFor(() => expect(result.current.binding?.state).toBe("active"));

    await act(() => result.current.setPaused(true));
    expect(result.current.binding?.state).toBe("paused");
    await act(() => result.current.setNotifications(true));
    expect(result.current.binding?.completionNotifications).toBe(true);
    await act(() => result.current.removeBinding());
    expect(result.current.binding).toBeNull();
  });

  it("unsubscribes from connector health when unmounted", async () => {
    const mock = mockService();
    const { unmount } = renderHook(() => useSessionImState("session-1", mock.service));
    await waitFor(() => expect(mock.service.subscribeLifecycle).toHaveBeenCalledOnce());

    unmount();

    expect(mock.unsubscribe).toHaveBeenCalledOnce();
  });
});
