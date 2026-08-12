// @vitest-environment jsdom

import type { ReactNode } from "react";
import { act, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Session } from "../types/agent";

const switchSession = vi.hoisted(() => vi.fn());
vi.mock("../services/runtime-agent-client", () => ({ agentService: { switchSession } }));

import { useSessionSwitch } from "./use-session-switch";

function session(id: string, archived = false): Session {
  return {
    id,
    title: id,
    agentId: "onepiece",
    interactionMode: "api",
    lifecycleState: "idle",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: null,
    projectPath: null,
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived,
    createdAt: "2026-08-11T00:00:00Z",
    updatedAt: "2026-08-11T00:00:00Z",
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, reject, resolve };
}

describe("useSessionSwitch", () => {
  let client: QueryClient;
  let onError: (reason: unknown, sessionId: string) => void;
  let wrapper: ({ children }: { children: ReactNode }) => ReactNode;

  beforeEach(() => {
    switchSession.mockReset();
    onError = vi.fn();
    client = new QueryClient({ defaultOptions: { mutations: { retry: false }, queries: { retry: false } } });
    wrapper = ({ children }) => <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  });

  it("publishes the selected session before persistence completes and preserves cached messages", async () => {
    const first = session("session-a");
    const second = session("session-b");
    const pending = deferred<Session>();
    client.setQueryData(["sessions", "active"], first);
    client.setQueryData(["messages", second.id, 50], ["cached message"]);
    switchSession.mockReturnValue(pending.promise);
    const { result } = renderHook(
      () => useSessionSwitch({ activeSessionId: first.id, onError }),
      { wrapper },
    );

    act(() => result.current(second));

    await waitFor(() => expect(client.getQueryData<Session>(["sessions", "active"])?.id).toBe(second.id));
    expect(client.getQueryData(["messages", second.id, 50])).toEqual(["cached message"]);
    expect(switchSession).toHaveBeenCalledWith(second.id);
    pending.resolve(second);
    await waitFor(() => expect(client.getQueryData<Session>(["sessions", "active"])?.id).toBe(second.id));
  });

  it("rolls back the latest failed switch and reports the failure", async () => {
    const first = session("session-a");
    const second = session("session-b");
    const pending = deferred<Session>();
    client.setQueryData(["sessions", "active"], first);
    switchSession.mockReturnValue(pending.promise);
    const { result } = renderHook(
      () => useSessionSwitch({ activeSessionId: first.id, onError }),
      { wrapper },
    );

    act(() => result.current(second));
    await waitFor(() => expect(client.getQueryData<Session>(["sessions", "active"])?.id).toBe(second.id));
    pending.reject(new Error("persistence failed"));

    await waitFor(() => expect(client.getQueryData<Session>(["sessions", "active"])?.id).toBe(first.id));
    expect(onError).toHaveBeenCalledWith(expect.any(Error), second.id);
  });

  it("keeps the most recent selection when older requests complete later", async () => {
    const first = session("session-a");
    const second = session("session-b");
    const third = session("session-c");
    const secondRequest = deferred<Session>();
    const thirdRequest = deferred<Session>();
    client.setQueryData(["sessions", "active"], first);
    switchSession.mockImplementation((id: string) => id === second.id ? secondRequest.promise : thirdRequest.promise);
    const { result } = renderHook(
      () => useSessionSwitch({ activeSessionId: first.id, onError }),
      { wrapper },
    );

    act(() => {
      result.current(second);
      result.current(third);
    });
    await waitFor(() => expect(client.getQueryData<Session>(["sessions", "active"])?.id).toBe(third.id));
    thirdRequest.resolve(third);
    secondRequest.resolve(second);

    await waitFor(() => expect(switchSession).toHaveBeenCalledTimes(2));
    expect(client.getQueryData<Session>(["sessions", "active"])?.id).toBe(third.id);
    expect(onError).not.toHaveBeenCalled();
  });

  it("does not roll back the most recent selection when an older request fails", async () => {
    const first = session("session-a");
    const second = session("session-b");
    const third = session("session-c");
    const secondRequest = deferred<Session>();
    const thirdRequest = deferred<Session>();
    client.setQueryData(["sessions", "active"], first);
    switchSession.mockImplementation((id: string) => id === second.id ? secondRequest.promise : thirdRequest.promise);
    const { result } = renderHook(
      () => useSessionSwitch({ activeSessionId: first.id, onError }),
      { wrapper },
    );

    act(() => {
      result.current(second);
      result.current(third);
    });
    thirdRequest.resolve(third);
    secondRequest.reject(new Error("stale persistence failure"));

    await waitFor(() => expect(switchSession).toHaveBeenCalledTimes(2));
    expect(client.getQueryData<Session>(["sessions", "active"])?.id).toBe(third.id);
    expect(onError).not.toHaveBeenCalled();
  });

  it("allows a rapid final selection that matches the session active before the burst", async () => {
    const first = session("session-a");
    const second = session("session-b");
    const secondRequest = deferred<Session>();
    const firstRequest = deferred<Session>();
    client.setQueryData(["sessions", "active"], first);
    switchSession.mockImplementation((id: string) => id === second.id ? secondRequest.promise : firstRequest.promise);
    const { result } = renderHook(
      () => useSessionSwitch({ activeSessionId: first.id, onError }),
      { wrapper },
    );

    act(() => {
      result.current(second);
      result.current(first);
    });

    await waitFor(() => expect(switchSession).toHaveBeenCalledTimes(2));
    expect(switchSession).toHaveBeenLastCalledWith(first.id);
    firstRequest.resolve(first);
    secondRequest.resolve(second);
    await waitFor(() => expect(client.getQueryData<Session>(["sessions", "active"])?.id).toBe(first.id));
  });

  it("does not persist the current or an archived session", () => {
    const first = session("session-a");
    const archived = session("session-b", true);
    client.setQueryData(["sessions", "active"], first);
    const { result } = renderHook(
      () => useSessionSwitch({ activeSessionId: first.id, onError }),
      { wrapper },
    );

    act(() => {
      result.current(first);
      result.current(archived);
    });

    expect(switchSession).not.toHaveBeenCalled();
    expect(client.getQueryData<Session>(["sessions", "active"])?.id).toBe(first.id);
  });
});
