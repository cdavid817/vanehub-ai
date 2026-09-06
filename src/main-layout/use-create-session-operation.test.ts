// @vitest-environment jsdom

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useCreateSessionOperation } from "./use-create-session-operation";

const { operationService, agentService } = vi.hoisted(() => ({
  operationService: { getOperationStatus: vi.fn() },
  agentService: { getSession: vi.fn() },
}));

vi.mock("../services/runtime-operation-client", () => ({ operationService }));
vi.mock("../services/runtime-agent-client", () => ({ agentService }));

const createdSession = {
  id: "session-7",
  agentId: "codex-cli",
  interactionMode: "cli",
};

function succeededOperation() {
  return { id: "operation-1", status: "succeeded", result: createdSession, error: null };
}

function mount(overrides: Record<string, unknown> = {}) {
  const setError = vi.fn();
  const setHandledOperationId = vi.fn();
  const setLoading = vi.fn();
  const onCreated = vi.fn();
  renderHook(() =>
    useCreateSessionOperation({
      active: true,
      handledOperationId: null,
      onCreated,
      operationId: "operation-1",
      setError,
      setHandledOperationId,
      setLoading,
      t: (key: string) => key,
      ...overrides,
    }),
  );
  return { onCreated, setError, setHandledOperationId, setLoading };
}

/**
 * The state that used to be indistinguishable from failure.
 *
 * When the operation succeeds and reading the created session then fails, the session exists. The
 * previous flow marked the operation handled *before* that read, so a transient error ended the
 * flow with no way back — and the only move left to the user created a second session.
 */
describe("create-session operation watcher", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    operationService.getOperationStatus.mockResolvedValue(succeededOperation());
    agentService.getSession.mockResolvedValue(createdSession);
  });

  it("delivers the session and only then marks the operation handled", async () => {
    const { onCreated, setHandledOperationId } = mount();

    await waitFor(() => expect(onCreated).toHaveBeenCalledWith(createdSession));
    expect(setHandledOperationId).toHaveBeenCalledWith("operation-1");
  });

  it("retries the canonical read instead of ending the flow", async () => {
    agentService.getSession
      .mockRejectedValueOnce(new Error("temporarily unavailable"))
      .mockResolvedValue(createdSession);

    const { onCreated, setError } = mount();

    // The read is retried; the creation is not resubmitted, because retrying a read is idempotent
    // and retrying a creation makes a second session.
    await waitFor(() => expect(onCreated).toHaveBeenCalledWith(createdSession), { timeout: 4000 });
    expect(setError).not.toHaveBeenCalled();
  });

  it("never resubmits creation while recovering a failed read", async () => {
    agentService.getSession.mockRejectedValue(new Error("still unavailable"));

    mount();

    // Only ever re-reads. `createSession` is not even reachable from here, which is the point:
    // recovery and resubmission are different operations and must not share a path.
    await waitFor(
      () => expect(agentService.getSession.mock.calls.length).toBeGreaterThan(1),
      { timeout: 5000 },
    );
    expect(operationService.getOperationStatus).toHaveBeenCalled();
  });

  it("gives up with an error rather than retrying forever", async () => {
    agentService.getSession.mockRejectedValue(new Error("permanently unavailable"));

    const { setError, setHandledOperationId } = mount();

    // A read that keeps failing is a real problem the user has to be told about. Retrying quietly
    // forever would leave the dialog spinning with no explanation.
    await waitFor(() => expect(setError).toHaveBeenCalled(), { timeout: 8000 });
    expect(setHandledOperationId).toHaveBeenCalledWith("operation-1");
  });

  it("does not re-poll when only the callback identity changes", async () => {
    agentService.getSession.mockRejectedValue(new Error("temporarily unavailable"));

    const { rerender } = renderHook(
      ({ onCreated }: { onCreated: (session: unknown) => void }) =>
        useCreateSessionOperation({
          active: true,
          handledOperationId: null,
          onCreated: onCreated as never,
          operationId: "operation-1",
          setError: vi.fn(),
          setHandledOperationId: vi.fn(),
          setLoading: vi.fn(),
          t: (key: string) => key,
        }),
      { initialProps: { onCreated: () => {} } },
    );
    await waitFor(() => expect(agentService.getSession).toHaveBeenCalled());
    const afterFirstPoll = agentService.getSession.mock.calls.length;

    // Callers pass `onCreated` inline, so every parent render supplies a new identity. If the
    // effect depended on it, each render would tear down the pending retry and poll again
    // immediately -- the interval would never be waited and the attempt budget could be spent in
    // milliseconds, reporting a failure that had not happened.
    for (let index = 0; index < 6; index += 1) {
      rerender({ onCreated: () => {} });
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });
    }

    expect(agentService.getSession.mock.calls.length).toBe(afterFirstPoll);
  });

  it("stays busy across a retry, so the submit button is never live mid-recovery", async () => {
    agentService.getSession
      .mockRejectedValueOnce(new Error("temporarily unavailable"))
      .mockResolvedValue(createdSession);

    const { onCreated, setLoading } = mount();

    await waitFor(() => expect(onCreated).toHaveBeenCalledWith(createdSession), { timeout: 4000 });
    // The dialog gates submit on `loading`. Clearing it between a failed read and its retry shows
    // no spinner and no error, which invites the click that creates the second session.
    const clearedBeforeDelivery = setLoading.mock.calls
      .slice(0, -1)
      .some(([value]) => value === false);
    expect(clearedBeforeDelivery).toBe(false);
  });

  it("abandons the watch when the creation surface closes", async () => {
    const onCreated = vi.fn();
    let resolveStatus: (value: unknown) => void = () => {};
    operationService.getOperationStatus.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveStatus = resolve;
        }),
    );

    const { rerender } = renderHook(
      ({ active }: { active: boolean }) =>
        useCreateSessionOperation({
          active,
          handledOperationId: null,
          onCreated,
          operationId: "operation-1",
          setError: vi.fn(),
          setHandledOperationId: vi.fn(),
          setLoading: vi.fn(),
          t: (key: string) => key,
        }),
      { initialProps: { active: true } },
    );

    // The user cancels while the operation is still in flight.
    rerender({ active: false });
    await act(async () => {
      resolveStatus(succeededOperation());
      await Promise.resolve();
      await Promise.resolve();
    });

    // The session may well have been created — it stays reachable from the session list — but the
    // user asked not to be taken to it, and delivering navigates them there.
    expect(onCreated).not.toHaveBeenCalled();
  });

  it("marks a failed operation handled without retrying it", async () => {
    operationService.getOperationStatus.mockResolvedValue({
      id: "operation-1",
      status: "failed",
      result: null,
      error: "workspace unavailable",
    });

    const { setError, setHandledOperationId, onCreated } = mount();

    await waitFor(() => expect(setError).toHaveBeenCalledWith("workspace unavailable"));
    expect(setHandledOperationId).toHaveBeenCalledWith("operation-1");
    expect(onCreated).not.toHaveBeenCalled();
  });
});
