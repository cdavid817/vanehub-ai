// @vitest-environment jsdom

import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useCommandCenterSearch } from "./use-command-center-search";
import type { WorkbenchSearchPage, WorkbenchSearchProvider, WorkbenchSearchResult } from "./command-center-types";

// A leaked fake-timer state from a failed assertion mid-test would hang every subsequent test's
// own real-timer waits — a global safety net rather than trusting each test's own cleanup path.
afterEach(() => vi.useRealTimers());

function result(overrides: Partial<WorkbenchSearchResult> = {}): WorkbenchSearchResult {
  return {
    key: "k",
    kind: "session",
    title: "Untitled",
    route: { destination: "sessions", sessionId: "s", creatingSession: false },
    ...overrides,
  };
}

function fakeProvider(id: string, page: (query: string) => Promise<WorkbenchSearchPage>): WorkbenchSearchProvider {
  return { id, supports: () => true, search: (request) => page(request.query) };
}

describe("useCommandCenterSearch", () => {
  it("does not search for an empty query", async () => {
    const search = vi.fn().mockResolvedValue({ items: [], nextCursor: null });
    // `providers` built once, outside the render callback: an inline array literal passed
    // straight into `renderHook`'s callback gets a fresh identity every render, and the hook's
    // ref-based read exists precisely so that no longer causes a render loop — built here as a
    // stable reference anyway, since a test relying solely on the hook's own defense would not
    // catch a regression in it.
    const providers = [fakeProvider("a", search)];
    renderHook(() => useCommandCenterSearch("", providers));
    await new Promise((resolve) => setTimeout(resolve, 300));
    expect(search).not.toHaveBeenCalled();
  });

  it("debounces before searching", async () => {
    // Fake timers rather than a real 150ms wait: DEBOUNCE_MS is 250, so a real wait only had a
    // 100ms margin against event-loop/CI-load jitter before this could flip `not.toHaveBeenCalled()`
    // into a false failure -- the exact "races real setTimeout delays" flakiness task 21.6 warns
    // about. `vi.advanceTimersByTime` makes both checkpoints exact regardless of real load.
    vi.useFakeTimers();
    const search = vi.fn().mockResolvedValue({ items: [result({ title: "auth" })], nextCursor: null });
    const providers = [fakeProvider("a", search)];
    // Starts empty: `useDebouncedValue` returns its *initial* value immediately, with no delay —
    // only subsequent changes are debounced. Starting from a non-empty query would fire a search
    // on mount, before any of the rerenders below ever happen.
    const { rerender } = renderHook(({ query }) => useCommandCenterSearch(query, providers), { initialProps: { query: "" } });
    rerender({ query: "a" });
    rerender({ query: "au" });
    rerender({ query: "aut" });
    rerender({ query: "auth" });

    await act(async () => { vi.advanceTimersByTime(150); });
    expect(search).not.toHaveBeenCalled();

    await act(async () => { vi.advanceTimersByTime(150); });
    expect(search).toHaveBeenCalledTimes(1);
    expect(search).toHaveBeenCalledWith("auth");
  });

  it("reports loading while a search is in flight, then not once it resolves", async () => {
    let resolveSearch: ((page: WorkbenchSearchPage) => void) | undefined;
    const providers = [fakeProvider("a", () => new Promise((resolve) => { resolveSearch = resolve; }))];
    const { result: hook } = renderHook(() => useCommandCenterSearch("auth", providers));
    await waitFor(() => expect(hook.current.loading).toBe(true));
    resolveSearch?.({ items: [result({ title: "auth" })], nextCursor: null });
    await waitFor(() => expect(hook.current.loading).toBe(false));
    expect(hook.current.results).toHaveLength(1);
  });

  it("merges and ranks results across multiple providers", async () => {
    const providerA = fakeProvider("a", () => Promise.resolve({ items: [result({ key: "a-1", title: "auth token" })], nextCursor: null }));
    const providerB = fakeProvider("b", () => Promise.resolve({ items: [result({ key: "b-1", title: "auth" })], nextCursor: null }));
    const providers = [providerA, providerB];
    const { result: hook } = renderHook(() => useCommandCenterSearch("auth", providers));
    await waitFor(() => expect(hook.current.results).toHaveLength(2));
    // "auth" is an exact title match, ranked above providerA's prefix match, regardless of which
    // provider returned it first — proves this is a real merge+rank, not just a concatenation.
    expect(hook.current.results.map((entry) => entry.key)).toEqual(["b-1", "a-1"]);
  });

  it("keeps a failed provider's rejection from blocking the others' results (partial-failure state)", async () => {
    const okProvider = fakeProvider("ok", () => Promise.resolve({ items: [result({ key: "ok-1", title: "auth" })], nextCursor: null }));
    const failingProvider = fakeProvider("failing", () => Promise.reject(new Error("network error")));
    const providers = [okProvider, failingProvider];
    const { result: hook } = renderHook(() => useCommandCenterSearch("auth", providers));
    await waitFor(() => expect(hook.current.loading).toBe(false));
    expect(hook.current.results.map((entry) => entry.key)).toEqual(["ok-1"]);
    expect(hook.current.failedProviderIds).toEqual(["failing"]);
  });

  it("discards a stale response that resolves after a newer query already superseded it", async () => {
    let resolveFirst: ((page: WorkbenchSearchPage) => void) | undefined;
    let resolveSecond: ((page: WorkbenchSearchPage) => void) | undefined;
    let call = 0;
    const providers = [fakeProvider("a", () => {
      call += 1;
      return new Promise<WorkbenchSearchPage>((resolve) => { if (call === 1) resolveFirst = resolve; else resolveSecond = resolve; });
    })];
    // Real timers throughout: past debounce naturally rather than fast-forwarding, so this test
    // never mixes fake- and real-timer waits within itself.
    const { result: hook, rerender } = renderHook(({ query }) => useCommandCenterSearch(query, providers), { initialProps: { query: "first" } });
    await waitFor(() => expect(call).toBe(1), { timeout: 1_000 });
    rerender({ query: "second" });
    await waitFor(() => expect(call).toBe(2), { timeout: 1_000 });

    // The second (newer) query's response arrives first, as it normally would.
    resolveSecond?.({ items: [result({ key: "second-result", title: "second" })], nextCursor: null });
    await waitFor(() => expect(hook.current.results).toHaveLength(1));
    expect(hook.current.results[0].key).toBe("second-result");

    // The stale first query's response arrives late — must not clobber the newer results.
    resolveFirst?.({ items: [result({ key: "first-result", title: "first" })], nextCursor: null });
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(hook.current.results.map((entry) => entry.key)).toEqual(["second-result"]);
  });

  it("clears results when the query is cleared back to empty", async () => {
    const providers = [fakeProvider("a", () => Promise.resolve({ items: [result({ title: "auth" })], nextCursor: null }))];
    const { result: hook, rerender } = renderHook(({ query }) => useCommandCenterSearch(query, providers), { initialProps: { query: "auth" } });
    await waitFor(() => expect(hook.current.results).toHaveLength(1));
    rerender({ query: "" });
    await waitFor(() => expect(hook.current.results).toHaveLength(0));
    expect(hook.current.loading).toBe(false);
  });
});
