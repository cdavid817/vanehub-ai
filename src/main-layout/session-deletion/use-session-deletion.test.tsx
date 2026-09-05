// @vitest-environment jsdom

import type { ReactNode } from "react";
import { act, renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Session } from "../../types/agent";
import type { SessionDeletionOperation, SessionDeletionPreview } from "../../types/session-deletion";

const service = vi.hoisted(() => ({
  previewSessionDeletion: vi.fn(),
  executeSessionDeletion: vi.fn(),
  getSessionDeletionOperation: vi.fn(),
  retrySessionDeletion: vi.fn(),
  deleteSession: vi.fn(),
}));
vi.mock("../../services/runtime-agent-client", () => ({ agentService: service }));

import { useSessionDeletion } from "./use-session-deletion";

function session(id: string, worktreePath: string | null = null): Session {
  return {
    id,
    title: `Title ${id}`,
    agentId: "codex-cli",
    interactionMode: "cli",
    personalizationMode: "standard",
    lifecycleState: "idle",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: worktreePath ?? "/repo",
    projectPath: "/repo",
    worktreePath,
    worktreeName: worktreePath ? "feature" : null,
    worktreeBranch: worktreePath ? "vanehub/feature" : null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: "2026-09-05T00:00:00Z",
    updatedAt: "2026-09-05T00:00:00Z",
  };
}

function preview(): SessionDeletionPreview {
  return {
    previewId: "pv-1",
    runtimeEffect: "native",
    createdAt: "t",
    expiresAt: "t",
    sessions: [{ sessionId: "s1", title: "Title s1", archived: false, active: false, workspaceKind: "worktree", worktreeKey: "wt-1", displayPath: "/repo-feature" }],
    worktrees: [{
      worktreeKey: "wt-1",
      worktreeId: "wt-1",
      displayPath: "/repo-feature",
      branch: "vanehub/feature",
      sessionIds: ["s1"],
      externalReferences: [],
      allowedPolicies: ["keep", "remove-safe"],
      blockers: [],
      checks: "complete",
      changes: { trackedModified: 0, staged: 0, conflicted: 0, untracked: 0 },
      ignored: null,
      requiresIgnoredAcknowledgement: false,
      origin: "ordinary_session",
      provenance: "verified",
      resourceStatus: "attached",
    }],
  };
}

function operation(outcome: SessionDeletionOperation["outcome"], overrides: Partial<SessionDeletionOperation["groups"][number]> = {}): SessionDeletionOperation {
  return {
    operationId: "op-1",
    requestId: "r",
    outcome,
    phase: outcome === "pending" ? "quiescing" : "completed",
    revision: 3,
    runtimeEffect: "native",
    createdAt: "t",
    updatedAt: "t",
    completedAt: null,
    groups: [{
      groupId: "g-1",
      worktreeKey: "wt-1",
      worktreeId: "wt-1",
      policy: "keep",
      sessionIds: ["s1"],
      status: outcome === "succeeded" ? "succeeded" : outcome === "pending" ? "running" : "awaiting_decision",
      phase: "completed",
      worktreeEffect: "retained",
      dbEffect: outcome === "succeeded" ? "deleted" : outcome === "pending" ? "pending" : "retained",
      errorCode: outcome === "succeeded" || outcome === "pending" ? null : "worktree_removal_refused",
      retainedPath: "/repo-feature",
      attempt: 1,
      revision: 2,
      ...overrides,
    }],
    errorCode: null,
    operationTaskId: null,
  };
}

describe("useSessionDeletion", () => {
  let client: QueryClient;
  let wrapper: ({ children }: { children: ReactNode }) => ReactNode;

  beforeEach(() => {
    vi.useRealTimers();
    for (const fn of Object.values(service)) fn.mockReset();
    client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    wrapper = ({ children }) => <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  });

  it("previews on request, defaults to keep, and never calls the legacy delete", async () => {
    service.previewSessionDeletion.mockResolvedValue(preview());
    const { result } = renderHook(() => useSessionDeletion(), { wrapper });
    act(() => result.current.request([session("s1", "/repo-feature")]));
    expect(result.current.state.status).toBe("loading");
    await waitFor(() => expect(result.current.state.status).toBe("ready"));
    expect(service.previewSessionDeletion).toHaveBeenCalledWith({ sessionIds: ["s1"] });
    const state = result.current.state;
    if (state.status !== "ready") throw new Error("expected ready");
    expect(state.choices["wt-1"].remove).toBe(false);
    expect(service.deleteSession).not.toHaveBeenCalled();
    // Cancelling before confirmation executes nothing.
    act(() => result.current.close());
    expect(result.current.state.status).toBe("closed");
    expect(service.executeSessionDeletion).not.toHaveBeenCalled();
  });

  it("executes with a stable request id, follows the operation, and refuses to close mid-flight", async () => {
    service.previewSessionDeletion.mockResolvedValue(preview());
    service.executeSessionDeletion.mockResolvedValue({ operationId: "op-1", runtimeEffect: "native", operationTaskId: null, existing: false });
    service.getSessionDeletionOperation
      .mockResolvedValueOnce(operation("pending"))
      .mockResolvedValue(operation("succeeded"));
    const { result } = renderHook(() => useSessionDeletion(), { wrapper });
    act(() => result.current.request([session("s1", "/repo-feature")]));
    await waitFor(() => expect(result.current.state.status).toBe("ready"));
    const ready = result.current.state;
    if (ready.status !== "ready") throw new Error("expected ready");
    act(() => result.current.toggleWorktree(ready.preview.worktrees[0]));
    await act(async () => { await result.current.confirm(); });
    expect(service.executeSessionDeletion).toHaveBeenCalledWith({
      requestId: ready.requestId,
      previewId: "pv-1",
      worktreeChoices: [{ worktreeKey: "wt-1", policy: "remove-safe" }],
    });
    expect(result.current.state.status).toBe("executing");
    act(() => result.current.close());
    expect(result.current.state.status).toBe("executing");
    await waitFor(() => expect(result.current.state.status).toBe("settled"), { timeout: 3_000 });
    const settled = result.current.state;
    if (settled.status !== "settled") throw new Error("expected settled");
    expect(settled.operation.outcome).toBe("succeeded");
  });

  it("keeps a refused execution on the dialog with its error and resets consent on refresh", async () => {
    service.previewSessionDeletion.mockResolvedValue(preview());
    service.executeSessionDeletion.mockRejectedValue(new Error("validation error: deletion_preview_expired"));
    const { result } = renderHook(() => useSessionDeletion(), { wrapper });
    act(() => result.current.request([session("s1", "/repo-feature")]));
    await waitFor(() => expect(result.current.state.status).toBe("ready"));
    const ready = result.current.state;
    if (ready.status !== "ready") throw new Error("expected ready");
    act(() => result.current.toggleWorktree(ready.preview.worktrees[0]));
    await act(async () => { await result.current.confirm(); });
    const failed = result.current.state;
    if (failed.status !== "ready") throw new Error("expected ready with error");
    expect(failed.error).toContain("deletion_preview_expired");
    expect(failed.choices["wt-1"].remove).toBe(true);
    act(() => result.current.refresh());
    await waitFor(() => expect(service.previewSessionDeletion).toHaveBeenCalledTimes(2));
    await waitFor(() => {
      const state = result.current.state;
      expect(state.status === "ready" && !state.choices["wt-1"].remove).toBe(true);
    });
  });

  it("retries a settled non-success through a fresh preview bound to the operation", async () => {
    service.previewSessionDeletion.mockResolvedValue(preview());
    service.executeSessionDeletion.mockResolvedValue({ operationId: "op-1", runtimeEffect: "native", operationTaskId: null, existing: false });
    service.getSessionDeletionOperation.mockResolvedValue(operation("awaiting_decision"));
    service.retrySessionDeletion.mockResolvedValue({ operationId: "op-1", runtimeEffect: "native", operationTaskId: null, existing: false });
    const { result } = renderHook(() => useSessionDeletion(), { wrapper });
    act(() => result.current.request([session("s1", "/repo-feature")]));
    await waitFor(() => expect(result.current.state.status).toBe("ready"));
    await act(async () => { await result.current.confirm(); });
    await waitFor(() => expect(result.current.state.status).toBe("settled"), { timeout: 3_000 });
    await act(async () => { await result.current.retry(); });
    await waitFor(() => expect(result.current.state.status).toBe("ready"));
    const ready = result.current.state;
    if (ready.status !== "ready") throw new Error("expected ready");
    expect(ready.retryOf).toEqual({ operationId: "op-1", expectedRevision: 3 });
    expect(ready.choices["wt-1"].remove).toBe(false);
    service.getSessionDeletionOperation.mockResolvedValue(operation("succeeded"));
    await act(async () => { await result.current.confirm(); });
    expect(service.retrySessionDeletion).toHaveBeenCalledWith(expect.objectContaining({
      operationId: "op-1",
      expectedRevision: 3,
      previewId: "pv-1",
      worktreeChoices: [{ worktreeKey: "wt-1", policy: "keep" }],
    }));
  });

  it("stops following an operation it can no longer read instead of staying locked", async () => {
    vi.useFakeTimers();
    try {
      service.previewSessionDeletion.mockResolvedValue(preview());
      service.executeSessionDeletion.mockResolvedValue({ operationId: "op-1", runtimeEffect: "native", operationTaskId: null, existing: false });
      service.getSessionDeletionOperation.mockRejectedValue(new Error("deletion_operation_not_found"));
      const { result } = renderHook(() => useSessionDeletion(), { wrapper });
      await act(async () => { result.current.request([session("s1", "/repo-feature")]); await Promise.resolve(); });
      await act(async () => { await vi.advanceTimersByTimeAsync(0); });
      expect(result.current.state.status).toBe("ready");
      await act(async () => { await result.current.confirm(); });
      expect(result.current.state.status).toBe("executing");
      // A few failed reads are tolerated: the dialog stays locked on the running operation.
      await act(async () => { await vi.advanceTimersByTimeAsync(1_500); });
      expect(result.current.state.status).toBe("executing");
      // Past the tolerance it gives up the lock and shows the last error; nothing is deleted
      // from here, the journal still owns the operation.
      await act(async () => { await vi.advanceTimersByTimeAsync(6_000); });
      const state = result.current.state;
      expect(state.status).toBe("preview-failed");
      if (state.status !== "preview-failed") throw new Error("expected preview-failed");
      expect(state.error).toContain("deletion_operation_not_found");
      act(() => result.current.close());
      expect(result.current.state.status).toBe("closed");
    } finally {
      vi.useRealTimers();
    }
  });

  it("retries a finalize-pending group without a new preview", async () => {
    service.previewSessionDeletion.mockResolvedValue(preview());
    service.executeSessionDeletion.mockResolvedValue({ operationId: "op-1", runtimeEffect: "native", operationTaskId: null, existing: false });
    service.getSessionDeletionOperation.mockResolvedValue(operation("needs_attention", {
      status: "finalize_pending",
      worktreeEffect: "removed",
      dbEffect: "pending",
      errorCode: "session_finalize_failed",
    }));
    service.retrySessionDeletion.mockResolvedValue({ operationId: "op-1", runtimeEffect: "native", operationTaskId: null, existing: false });
    const { result } = renderHook(() => useSessionDeletion(), { wrapper });
    act(() => result.current.request([session("s1", "/repo-feature")]));
    await waitFor(() => expect(result.current.state.status).toBe("ready"));
    await act(async () => { await result.current.confirm(); });
    await waitFor(() => expect(result.current.state.status).toBe("settled"), { timeout: 3_000 });
    service.getSessionDeletionOperation.mockResolvedValue(operation("succeeded"));
    await act(async () => { await result.current.retry(); });
    expect(service.previewSessionDeletion).toHaveBeenCalledTimes(1);
    expect(service.retrySessionDeletion).toHaveBeenCalledWith(expect.objectContaining({ operationId: "op-1", worktreeChoices: [] }));
    expect(service.retrySessionDeletion.mock.calls[0][0].previewId).toBeUndefined();
  });
});
