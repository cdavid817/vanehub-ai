// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook } from "@testing-library/react";
import { createElement, type ReactNode } from "react";
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
 * A busy run emits a transition per span start and finish — dozens per second for real work.
 *
 * These assert *invalidation*, not a counter. The distinction is the whole point of the change
 * they cover: a counter in a query key makes each refresh a different query, which arrives empty
 * and takes the reader's loaded pages, selection and open drawer with it. An invalidation refetches
 * the same query and leaves all of that standing. A test that only counted refreshes could not
 * tell the two apart, and the previous one did exactly that.
 */
describe("trace live refresh", () => {
  let listeners: ((value: TraceTransitionNotice) => void)[] = [];
  let released = 0;
  let client: QueryClient;
  let invalidate: ReturnType<typeof vi.fn<(filters: { queryKey: unknown[] }) => Promise<void>>>;

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

  function mount(runId: string | null = "run-1", sessionId: string | null = "session-1") {
    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(QueryClientProvider, { client }, children);
    return renderHook(
      (props: { isVisible: boolean }) =>
        useTraceLiveRefresh({ isVisible: props.isVisible, runIds: [runId], sessionId, subscribe }),
      { initialProps: { isVisible: true }, wrapper },
    );
  }

  /** The keys invalidated so far, as plain arrays. */
  function invalidatedKeys(): unknown[][] {
    return invalidate.mock.calls.map((call) => call[0].queryKey);
  }

  beforeEach(() => {
    vi.useFakeTimers();
    listeners = [];
    released = 0;
    client = new QueryClient();
    invalidate = vi.fn(async () => {});
    // Spied rather than observed through refetch counts: the assertion is about *which query*
    // is invalidated, and a refetch count cannot distinguish that from a key change.
    client.invalidateQueries = invalidate as unknown as QueryClient["invalidateQueries"];
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("coalesces a burst into one invalidation", () => {
    mount();

    for (let index = 0; index < 20; index += 1) emit(notice({ spanId: `span-${index}` }));
    settle();

    // Twenty transitions, one read. Without this the query refetches faster than it can answer.
    expect(invalidatedKeys()).toEqual([["execution-timeline", "run-1"]]);
  });

  it("waits for the burst to stop before invalidating", () => {
    mount();

    emit(notice());
    act(() => {
      vi.advanceTimersByTime(TRACE_REFRESH_WINDOW_MS - 50);
    });

    // Trailing rather than leading: the last transition in a burst is the one whose state the
    // refetch should return.
    expect(invalidate).not.toHaveBeenCalled();
    settle();
    expect(invalidate).toHaveBeenCalledTimes(1);
  });

  it("invalidates by data identity, never by a changing key", () => {
    mount();

    emit(notice());
    settle();
    emit(notice());
    settle();

    // Both bursts name the same query. A key that changed per refresh would make the second one a
    // different query, with no pages, no selection and an unmounted viewport.
    expect(invalidatedKeys()).toEqual([
      ["execution-timeline", "run-1"],
      ["execution-timeline", "run-1"],
    ]);
  });

  it("ignores a span transition belonging to another run", () => {
    mount();

    emit(notice({ runId: "run-2" }));
    settle();

    // One busy background run would otherwise keep a reader's open timeline in permanent motion.
    expect(invalidate).not.toHaveBeenCalled();
  });

  it("refreshes a compared run that advances on its own", () => {
    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(QueryClientProvider, { client }, children);
    renderHook(
      () =>
        useTraceLiveRefresh({
          isVisible: true,
          // Selected run plus the compared one: two open timelines, not one.
          runIds: ["run-1", "run-2"],
          sessionId: "session-1",
          subscribe,
        }),
      { wrapper },
    );

    emit(notice({ runId: "run-2" }));
    settle();

    // Watching only the selected run left a comparison frozen at whatever it held when opened,
    // with nothing on screen saying the other side had moved on.
    expect(invalidatedKeys()).toEqual([["execution-timeline", "run-2"]]);
  });

  it("refreshes both open timelines from one burst", () => {
    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(QueryClientProvider, { client }, children);
    renderHook(
      () =>
        useTraceLiveRefresh({
          isVisible: true,
          runIds: ["run-1", "run-2"],
          sessionId: "session-1",
          subscribe,
        }),
      { wrapper },
    );

    emit(notice({ runId: "run-1" }));
    emit(notice({ runId: "run-2" }));
    settle();

    expect(invalidatedKeys()).toEqual([
      ["execution-timeline", "run-1"],
      ["execution-timeline", "run-2"],
    ]);
  });

  it("separates the run list from the open timeline", () => {
    mount();

    emit(notice({ affectsRunList: false }));
    settle();

    // A span finishing changes the open timeline and not the list of runs. Re-reading the list once
    // per span is how a busy run makes the whole panel unusable.
    expect(invalidatedKeys()).toEqual([["execution-timeline", "run-1"]]);
  });

  it("invalidates the run list for a run transition, scoped to the visible session", () => {
    mount();

    emit(notice({ kind: "run-started", runId: "run-9", affectsRunList: true, spanId: undefined }));
    settle();

    // Scoped by key: a session whose list is not on screen is not re-read. The notice itself
    // carries no session id, which is exactly why nothing user-facing is derived from it here.
    expect(invalidatedKeys()).toEqual([["execution-runs", "session-1"]]);
  });

  it("does not subscribe at all while the panel is hidden", () => {
    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(QueryClientProvider, { client }, children);
    renderHook(
      () =>
        useTraceLiveRefresh({
          isVisible: false,
          runIds: ["run-1"],
          sessionId: "session-1",
          subscribe,
        }),
      { wrapper },
    );

    // A hidden panel that kept refetching would spend a query per transition on a view nobody is
    // looking at — and it re-reads on becoming visible anyway.
    expect(listeners).toHaveLength(0);
  });

  it("releases the subscription when the panel is hidden", () => {
    const { rerender } = mount();
    expect(listeners).toHaveLength(1);

    rerender({ isVisible: false });

    expect(released).toBe(1);
    expect(listeners).toHaveLength(0);
  });

  it("drops a pending burst rather than firing it after being hidden", () => {
    const { rerender } = mount();
    emit(notice());

    rerender({ isVisible: false });
    settle();

    // Becoming visible again re-reads from scratch, so carrying the debt across would only
    // produce a duplicate read of a timeline nobody was watching.
    expect(invalidate).not.toHaveBeenCalled();
  });

  it("does nothing when there is no stream to subscribe to", () => {
    const wrapper = ({ children }: { children: ReactNode }) =>
      createElement(QueryClientProvider, { client }, children);
    renderHook(
      () =>
        useTraceLiveRefresh({
          isVisible: true,
          runIds: ["run-1"],
          sessionId: "session-1",
          subscribe: null,
        }),
      { wrapper },
    );

    settle();

    expect(invalidate).not.toHaveBeenCalled();
  });
});
