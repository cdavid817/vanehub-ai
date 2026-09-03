/** @vitest-environment jsdom */
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { agentService } from "../services/runtime-agent-client";
import type { FileContent } from "../types/session-workspace";
import { useFilePreview } from "./use-file-preview";

function file(path: string, content: string): FileContent {
  return {
    path,
    name: path.split("/").pop() ?? path,
    status: "text",
    size: content.length,
    content,
    encoding: "utf-8",
    newline: "lf",
  };
}

let read: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  read = vi.spyOn(agentService, "readSessionFile");
});

afterEach(() => {
  vi.restoreAllMocks();
});

function harness() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return { client, wrapper };
}

function mount(initialPath: string | null) {
  const { client, wrapper } = harness();
  const rendered = renderHook(
    ({ path }: { path: string | null }) => useFilePreview("session-1", path),
    { initialProps: { path: initialPath }, wrapper },
  );
  return { ...rendered, client };
}

describe("useFilePreview", () => {
  it("has nothing to show before anything is selected", () => {
    const { result } = mount(null);

    expect(result.current.shown).toBeNull();
    expect(result.current.isEmpty).toBe(true);
    expect(result.current.status.kind).toBe("current");
  });

  it("keeps the previous file on screen while another one loads", async () => {
    read.mockResolvedValueOnce(file("a.rs", "first"));
    const { rerender, result } = mount("a.rs");
    await waitFor(() => expect(result.current.shown?.path).toBe("a.rs"));

    // Never resolves, so the second file is genuinely still loading.
    read.mockImplementation(() => new Promise<FileContent>(() => {}));
    rerender({ path: "b.rs" });

    // The reader does not lose the file they were reading to a request that has not finished.
    await waitFor(() => expect(result.current.status.kind).toBe("loading"));
    expect(result.current.shown?.path).toBe("a.rs");
    expect(result.current.isEmpty).toBe(false);
  });

  it("names the file it is waiting for, not the one it is showing", async () => {
    read.mockResolvedValueOnce(file("a.rs", "first"));
    const { rerender, result } = mount("a.rs");
    await waitFor(() => expect(result.current.shown?.path).toBe("a.rs"));

    read.mockImplementation(() => new Promise<FileContent>(() => {}));
    rerender({ path: "b.rs" });
    await waitFor(() => expect(result.current.status.kind).toBe("loading"));

    // Content that stayed while its label changed would be the same thing as showing the wrong
    // file. The status names the pending one so the header can keep naming the visible one.
    expect(result.current.status).toEqual({ kind: "loading", pendingPath: "b.rs" });
  });

  it("keeps the previous file when the next one cannot be read", async () => {
    read.mockResolvedValueOnce(file("a.rs", "first"));
    const { rerender, result } = mount("a.rs");
    await waitFor(() => expect(result.current.shown?.path).toBe("a.rs"));

    read.mockRejectedValue(new Error("gone"));
    rerender({ path: "b.rs" });

    // The case React Query's `keepPreviousData` does not cover: on failure the query has no data,
    // and without explicit retention the reader loses the file they were reading because a
    // different one could not be read.
    await waitFor(() => expect(result.current.status.kind).toBe("failed"));
    expect(result.current.shown?.path).toBe("a.rs");
    expect(result.current.isEmpty).toBe(false);
  });

  it("reports a failure as the whole answer when there is nothing to fall back to", async () => {
    read.mockRejectedValue(new Error("gone"));
    const { result } = mount("a.rs");

    await waitFor(() => expect(result.current.status.kind).toBe("failed"));
    expect(result.current.shown).toBeNull();
    expect(result.current.isEmpty).toBe(true);
  });

  it("says a file is being re-read rather than that it is loading", async () => {
    read.mockResolvedValue(file("a.rs", "first"));
    const { client, result } = mount("a.rs");
    await waitFor(() => expect(result.current.shown?.path).toBe("a.rs"));

    read.mockImplementation(() => new Promise<FileContent>(() => {}));
    void client.invalidateQueries();

    // "Loading" would suggest the panel is empty when it is not, and the content on screen is
    // still this file's — probably still right, just possibly out of date.
    await waitFor(() => expect(result.current.status.kind).toBe("refreshing"));
    expect(result.current.shown?.path).toBe("a.rs");
  });

  it("does not carry a file across a session change", async () => {
    read.mockResolvedValue(file("a.rs", "first"));
    const { result, rerender } = renderHook(
      ({ path, sessionId }: { path: string; sessionId: string }) =>
        useFilePreview(sessionId, path),
      { initialProps: { path: "a.rs", sessionId: "session-1" }, ...harness() },
    );
    await waitFor(() => expect(result.current.shown?.path).toBe("a.rs"));

    read.mockImplementation(() => new Promise<FileContent>(() => {}));
    rerender({ path: "a.rs", sessionId: "session-2" });

    // Showing one workspace's content under another's tree is the one retention that is never an
    // improvement.
    await waitFor(() => expect(result.current.isEmpty).toBe(true));
    expect(result.current.shown).toBeNull();
  });
});
