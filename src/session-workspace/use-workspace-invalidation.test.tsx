/** @vitest-environment jsdom */
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { agentService } from "../services/runtime-agent-client";
import type { WorkspaceInvalidationNotice } from "../types/session-workspace-inspection";
import { useWorkspaceInvalidation } from "./use-workspace-invalidation";
import { workspaceQueryKeys } from "./workspace-query-keys";

let publish: ((notice: WorkspaceInvalidationNotice) => void) | null = null;
let release: () => void;
let releaseCount = 0;
let subscribe: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  releaseCount = 0;
  release = () => {
    releaseCount += 1;
  };
  // Held in a variable rather than re-read off the service. `agentService` resolves its methods on
  // access, so `agentService.subscribeWorkspaceInvalidation` is not the same function object twice
  // and an assertion against it would be asserting about something that was never called.
  subscribe = vi
    .spyOn(agentService, "subscribeWorkspaceInvalidation")
    .mockImplementation(async (handler) => {
      publish = handler;
      return release;
    });
});

afterEach(() => {
  publish = null;
  vi.restoreAllMocks();
});

function harness() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  // Recorded rather than spied on for the keys: the spy's own call type is loose enough that
  // reading a key off it needs a cast, and a cast in a test is a place an assertion can quietly
  // stop meaning what it says.
  const invalidatedKeys: string[] = [];
  const invalidate = vi
    .spyOn(client, "invalidateQueries")
    .mockImplementation(async (filters?: { queryKey?: readonly unknown[] }) => {
      invalidatedKeys.push(JSON.stringify(filters?.queryKey));
    });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return { client, invalidate, invalidatedKeys, wrapper };
}

function notice(overrides: Partial<WorkspaceInvalidationNotice> = {}): WorkspaceInvalidationNotice {
  return {
    sessionId: "session-1",
    source: "execution-evidence",
    scope: "path",
    relativePath: "src/main.rs",
    change: "modified",
    sequence: 1,
    occurredAt: "2026-08-26T09:00:00Z",
    ...overrides,
  };
}

describe("useWorkspaceInvalidation", () => {
  it("invalidates the queries a notice implicates", async () => {
    const { invalidatedKeys, wrapper } = harness();
    renderHook(() => useWorkspaceInvalidation("session-1"), { wrapper });
    await waitFor(() => expect(publish).not.toBeNull());

    publish?.(notice());

    expect(invalidatedKeys).toContain(
      JSON.stringify(workspaceQueryKeys.directory("session-1", "src")),
    );
    expect(invalidatedKeys).toContain(JSON.stringify(workspaceQueryKeys.gitStatus("session-1")));
    // Not the whole session, and not another directory. The targeting is the feature; a broad
    // refresh here would collapse every expanded folder on each agent write.
    expect(invalidatedKeys).not.toContain(JSON.stringify(workspaceQueryKeys.session("session-1")));
    expect(invalidatedKeys).not.toContain(
      JSON.stringify(workspaceQueryKeys.directory("session-1", "docs")),
    );
  });

  it("ignores a notice about another session", async () => {
    const { invalidate, wrapper } = harness();
    renderHook(() => useWorkspaceInvalidation("session-1"), { wrapper });
    await waitFor(() => expect(publish).not.toBeNull());

    publish?.(notice({ sessionId: "session-2" }));

    // The native side publishes for every session because it cannot know which one is on screen.
    // Acting on all of them would refetch panels that are not showing anything.
    expect(invalidate).not.toHaveBeenCalled();
  });

  it("releases the subscription when the session changes", async () => {
    const { wrapper } = harness();
    const { rerender } = renderHook(
      ({ sessionId }: { sessionId: string }) => useWorkspaceInvalidation(sessionId),
      { initialProps: { sessionId: "session-1" }, wrapper },
    );
    await waitFor(() => expect(publish).not.toBeNull());

    rerender({ sessionId: "session-2" });

    // A leak here is invisible: the old listener keeps running, keeps matching nothing, and the
    // count grows by one on every session switch for the life of the process.
    await waitFor(() => expect(releaseCount).toBe(1));
  });

  it("does nothing at all without a session", async () => {
    const { wrapper } = harness();
    renderHook(() => useWorkspaceInvalidation(null), { wrapper });

    expect(subscribe).not.toHaveBeenCalled();
  });

  it("survives a build with no notice channel", async () => {
    vi.spyOn(agentService, "subscribeWorkspaceInvalidation").mockRejectedValue(
      new Error("no channel"),
    );
    const { wrapper } = harness();

    // A rejected subscription is a build where nothing changes on its own — the browser adapter.
    // Reads still work and refresh still works, so there is nothing to raise.
    expect(() => renderHook(() => useWorkspaceInvalidation("session-1"), { wrapper })).not.toThrow();
  });
});
