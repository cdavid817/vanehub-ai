// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TraceTransitionNotice } from "../types/trace-transition";
import { TRACE_REFRESH_WINDOW_MS, useTraceLiveRefresh } from "./use-trace-live-refresh";

function notice(overrides: Partial<TraceTransitionNotice> = {}): TraceTransitionNotice {
  return {
    kind: "span-finished",
    runId: "run-1",
    traceId: "trace-1",
    spanId: "span-1",
    status: "succeeded",
    affectsRunList: false,
    ...overrides,
  };
}

/**
 * A busy run emits a transition per span start and per span finish — dozens per second for a run
 * doing real work. Refetching on each would put the timeline query into a loop that never settles,
 * and every response would be stale before it rendered.
 */
describe("trace live refresh", () => {
  let listeners: ((value: TraceTransitionNotice) => void)[] = [];
  let released = 0;

  const subscribe = (listener: (value: TraceTransitionNotice) => void) => {
    listeners.push(listener);
    return () => {
      released += 1;
      listeners = listeners.filter((item) => item !== listener);
    };
  };

  function emit(value: TraceTransitionNotice) {
    act(() => {
      for (const listener of [...listeners]) listener(value);
    });
  }

  function settle() {
    act(() => {
      vi.advanceTimersByTime(TRACE_REFRESH_WINDOW_MS + 1);
    });
  }

  beforeEach(() => {
    vi.useFakeTimers();
    listeners = [];
    released = 0;
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("coalesces a burst into one refresh", () => {
    const { result } = renderHook(() =>
      useTraceLiveRefresh({ isVisible: true, runId: "run-1", subscribe }),
    );

    for (let index = 0; index < 20; index += 1) emit(notice({ spanId: `span-${index}` }));
    settle();

    // Twenty transitions, one read. Without this the query refetches faster than it can answer.
    expect(result.current.refreshToken).toBe(1);
  });

  it("waits for the burst to stop before refreshing", () => {
    const { result } = renderHook(() =>
      useTraceLiveRefresh({ isVisible: true, runId: "run-1", subscribe }),
    );

    emit(notice());
    act(() => { vi.advanceTimersByTime(TRACE_REFRESH_WINDOW_MS - 50); });

    // Trailing rather than leading: the last transition in a burst is the one whose state the
    // refetch should return.
    expect(result.current.refreshToken).toBe(0);
    settle();
    expect(result.current.refreshToken).toBe(1);
  });

  it("refreshes again for a later burst", () => {
    const { result } = renderHook(() =>
      useTraceLiveRefresh({ isVisible: true, runId: "run-1", subscribe }),
    );

    emit(notice());
    settle();
    emit(notice());
    settle();

    expect(result.current.refreshToken).toBe(2);
  });

  it("ignores a span transition belonging to another run", () => {
    const { result } = renderHook(() =>
      useTraceLiveRefresh({ isVisible: true, runId: "run-1", subscribe }),
    );

    emit(notice({ runId: "run-2" }));
    settle();

    // One busy background run would otherwise keep a reader's open timeline in permanent motion.
    expect(result.current.refreshToken).toBe(0);
  });

  it("separates the run list from the open timeline", () => {
    const { result } = renderHook(() =>
      useTraceLiveRefresh({ isVisible: true, runId: "run-1", subscribe }),
    );

    emit(notice({ affectsRunList: false }));
    settle();

    // A span finishing changes the open timeline and not the list of runs. Re-reading the list once
    // per span is how a busy run makes the whole panel unusable.
    expect(result.current.refreshToken).toBe(1);
    expect(result.current.runListToken).toBe(0);
  });

  it("refreshes the run list for a run transition, even from another run", () => {
    const { result } = renderHook(() =>
      useTraceLiveRefresh({ isVisible: true, runId: "run-1", subscribe }),
    );

    emit(notice({ kind: "run-started", runId: "run-9", affectsRunList: true, spanId: undefined }));
    settle();

    // A new run belongs in the list whether or not the reader is looking at it.
    expect(result.current.runListToken).toBe(1);
    expect(result.current.refreshToken).toBe(0);
  });

  it("does not subscribe at all while the panel is hidden", () => {
    renderHook(() => useTraceLiveRefresh({ isVisible: false, runId: "run-1", subscribe }));

    // A hidden panel that kept refetching would spend a query per transition on a view nobody is
    // looking at — and it re-reads on becoming visible anyway.
    expect(listeners).toHaveLength(0);
  });

  it("releases the subscription when the panel is hidden", () => {
    const { rerender } = renderHook(
      ({ isVisible }) => useTraceLiveRefresh({ isVisible, runId: "run-1", subscribe }),
      { initialProps: { isVisible: true } },
    );
    expect(listeners).toHaveLength(1);

    rerender({ isVisible: false });

    expect(released).toBe(1);
    expect(listeners).toHaveLength(0);
  });

  it("drops a pending burst rather than firing it after being hidden", () => {
    const { result, rerender } = renderHook(
      ({ isVisible }) => useTraceLiveRefresh({ isVisible, runId: "run-1", subscribe }),
      { initialProps: { isVisible: true } },
    );
    emit(notice());

    rerender({ isVisible: false });
    settle();

    // Becoming visible again re-reads from scratch, so carrying the debt across would only
    // produce a duplicate read of a timeline nobody was watching.
    expect(result.current.refreshToken).toBe(0);
  });

  it("does nothing when there is no stream to subscribe to", () => {
    const { result } = renderHook(() =>
      useTraceLiveRefresh({ isVisible: true, runId: "run-1", subscribe: null }),
    );

    settle();

    expect(result.current.refreshToken).toBe(0);
    expect(result.current.runListToken).toBe(0);
  });
});
