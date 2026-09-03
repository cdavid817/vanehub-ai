// @vitest-environment jsdom

import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ExecutionTargetKind, ExecutionTargetOption, ExecutionTargetProviders, ExecutionTargetSearch } from "./execution-target-providers";
import { useExecutionTargetSearch } from "./use-execution-target-search";

afterEach(() => vi.useRealTimers());

function option(overrides: Partial<ExecutionTargetOption> = {}): ExecutionTargetOption {
  return { id: "loop-1", title: "Fix auth loop", projectPath: null, statusKey: "loops.definition.enabled", statusTone: "success", ...overrides };
}

// Built once outside the render callback -- an inline object literal passed straight into
// `renderHook`'s own callback gets a fresh identity every render, and the hook's ref-based read
// exists precisely so that no longer causes a render loop (use-command-center-search.ts's own
// doc comment). Built here as a stable reference anyway, since a test relying solely on the
// hook's own defense would not catch a regression in it.
function fakeProviders(loop: ExecutionTargetSearch): ExecutionTargetProviders {
  return { loop, work_item: loop, session: loop, run: loop };
}

describe("useExecutionTargetSearch", () => {
  it("debounces before searching", async () => {
    // Fake timers rather than a real 150ms wait: DEBOUNCE_MS is 250, so a real wait only has a
    // 100ms margin against event-loop/CI-load jitter -- exactly the "races real setTimeout
    // delays" flakiness task 21.6 warns about, not a hypothetical one (a slow tick can plausibly
    // eat 100ms). `vi.advanceTimersByTime` makes both checkpoints exact regardless of real load.
    vi.useFakeTimers();
    const search = vi.fn().mockResolvedValue([option()]);
    const providers = fakeProviders(search);
    const { rerender } = renderHook(
      ({ query }) => useExecutionTargetSearch("loop", query, providers),
      { initialProps: { query: "" } },
    );
    rerender({ query: "a" });
    rerender({ query: "au" });
    rerender({ query: "auth" });

    await act(async () => { vi.advanceTimersByTime(150); });
    // The initial empty query fires immediately (useDebouncedValue returns its initial value with
    // no delay) -- only the later "a"/"au"/"auth" edits are debounced into a single call.
    expect(search).toHaveBeenCalledTimes(1);

    await act(async () => { vi.advanceTimersByTime(150); });
    expect(search).toHaveBeenCalledTimes(2);
    expect(search).toHaveBeenLastCalledWith("auth");
  });

  it("reports loading while a search is in flight, then not once it resolves", async () => {
    let resolveSearch: ((options: ExecutionTargetOption[]) => void) | undefined;
    const providers = fakeProviders(() => new Promise((resolve) => { resolveSearch = resolve; }));
    const { result } = renderHook(() => useExecutionTargetSearch("loop", "auth", providers));
    await waitFor(() => expect(result.current.loading).toBe(true));
    resolveSearch?.([option()]);
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.options).toHaveLength(1);
  });

  it("surfaces a rejected search as a display-ready error message, not a thrown exception", async () => {
    const providers = fakeProviders(() => Promise.reject(new Error("service unavailable")));
    const { result } = renderHook(() => useExecutionTargetSearch("loop", "auth", providers));
    await waitFor(() => expect(result.current.error).toBe("service unavailable"));
    expect(result.current.options).toEqual([]);
  });

  it("re-searches when the kind changes even if the query stays the same", async () => {
    const search = vi.fn().mockResolvedValue([option()]);
    const providers = fakeProviders(search);
    const { rerender } = renderHook(
      ({ kind }: { kind: ExecutionTargetKind }) => useExecutionTargetSearch(kind, "auth", providers),
      { initialProps: { kind: "loop" } },
    );
    await waitFor(() => expect(search).toHaveBeenCalledTimes(1));
    rerender({ kind: "run" });
    await waitFor(() => expect(search).toHaveBeenCalledTimes(2));
  });

  it("discards a stale response that resolves after a newer query already superseded it", async () => {
    let resolveFirst: ((options: ExecutionTargetOption[]) => void) | undefined;
    let resolveSecond: ((options: ExecutionTargetOption[]) => void) | undefined;
    let call = 0;
    const providers = fakeProviders(() => {
      call += 1;
      return new Promise<ExecutionTargetOption[]>((resolve) => { if (call === 1) resolveFirst = resolve; else resolveSecond = resolve; });
    });
    const { result, rerender } = renderHook(
      ({ query }) => useExecutionTargetSearch("loop", query, providers),
      { initialProps: { query: "first" } },
    );
    await waitFor(() => expect(call).toBe(1), { timeout: 1_000 });
    rerender({ query: "second" });
    await waitFor(() => expect(call).toBe(2), { timeout: 1_000 });

    resolveSecond?.([option({ id: "loop-2", title: "second" })]);
    await waitFor(() => expect(result.current.options).toHaveLength(1));
    expect(result.current.options[0].id).toBe("loop-2");

    resolveFirst?.([option({ id: "loop-1", title: "first" })]);
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(result.current.options.map((entry) => entry.id)).toEqual(["loop-2"]);
  });
});
