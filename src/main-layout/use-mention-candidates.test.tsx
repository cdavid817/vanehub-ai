// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { searchSessionFiles } = vi.hoisted(() => ({ searchSessionFiles: vi.fn() }));
vi.mock("../services/runtime-agent-client", () => ({ agentService: { searchSessionFiles } }));

import { useMentionCandidates } from "./use-mention-candidates";

const listing = (paths: string[]) => ({
  context: { availability: "available" as const, rootName: "demo", reason: null },
  items: paths.map((path) => ({ name: path.split("/").pop() ?? path, path })),
  truncated: false,
});

function createWrapper() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: ReactNode }) => <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

describe("useMentionCandidates", () => {
  beforeEach(() => {
    searchSessionFiles.mockReset();
    searchSessionFiles.mockResolvedValue(listing(["src/session_search.rs"]));
  });

  it("returns ranked candidates for the active mention token", async () => {
    const { result } = renderHook(() => useMentionCandidates("session-1", "@session"), { wrapper: createWrapper() });
    await waitFor(() => expect(result.current).toHaveLength(1));
    expect(result.current[0].path).toBe("src/session_search.rs");
    expect(searchSessionFiles).toHaveBeenCalledWith("session-1", "session", 8);
  });

  it("does not issue a search for intermediate keystrokes", async () => {
    const { rerender } = renderHook(({ draft }) => useMentionCandidates("session-1", draft), {
      initialProps: { draft: "@a" },
      wrapper: createWrapper(),
    });
    rerender({ draft: "@ab" });
    rerender({ draft: "@abc" });
    await waitFor(() => expect(searchSessionFiles).toHaveBeenLastCalledWith("session-1", "abc", 8));
    const queries = searchSessionFiles.mock.calls.map((call) => call[1]);
    expect(queries).not.toContain("ab");
  });

  it("drops candidates once the mention token is no longer at the caret", async () => {
    const { result, rerender } = renderHook(({ draft }) => useMentionCandidates("session-1", draft), {
      initialProps: { draft: "@session" },
      wrapper: createWrapper(),
    });
    await waitFor(() => expect(result.current).toHaveLength(1));
    rerender({ draft: "@session and then prose" });
    expect(result.current).toEqual([]);
  });

  it("searches on the path portion while a line range is being typed", async () => {
    renderHook(() => useMentionCandidates("session-1", "@src/utils.rs:10-50"), { wrapper: createWrapper() });
    // Querying the whole token would empty completion the moment the user types `:`.
    await waitFor(() => expect(searchSessionFiles).toHaveBeenCalledWith("session-1", "src/utils.rs", 8));
  });

  it("does not reach the native runtime without an active session", () => {
    renderHook(() => useMentionCandidates(null, "@session"), { wrapper: createWrapper() });
    expect(searchSessionFiles).not.toHaveBeenCalled();
  });
});
