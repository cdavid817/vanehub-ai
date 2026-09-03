// @vitest-environment jsdom

import { renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useMissionControlPolling } from "./use-mission-control-polling";

afterEach(() => vi.useRealTimers());

describe("useMissionControlPolling", () => {
  it("widens the interval on repeated no-op reconciles, and resets it the instant one reports a real change", async () => {
    vi.useFakeTimers();
    const reconcile = vi.fn<() => Promise<boolean>>()
      .mockResolvedValueOnce(false) // tick1 @2000ms: no change -> backoff to 4000ms
      .mockResolvedValueOnce(false) // tick2 @6000ms: no change -> backoff to 8000ms
      .mockResolvedValueOnce(true) // tick3 @14000ms: real change -> reset to 2000ms
      .mockResolvedValue(false);
    renderHook(() => useMissionControlPolling(reconcile));

    await vi.advanceTimersByTimeAsync(2_000);
    expect(reconcile).toHaveBeenCalledTimes(1);

    // Backoff doubled to 4000ms -- the next tick is not due at a plain +2000ms mark anymore.
    await vi.advanceTimersByTimeAsync(2_000);
    expect(reconcile).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(2_000);
    expect(reconcile).toHaveBeenCalledTimes(2);

    // Backoff doubled again to 8000ms.
    await vi.advanceTimersByTimeAsync(7_999);
    expect(reconcile).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(1);
    expect(reconcile).toHaveBeenCalledTimes(3);

    // tick3 reported a real change -- back to the base 2000ms cadence, not a continued widen.
    await vi.advanceTimersByTimeAsync(2_000);
    expect(reconcile).toHaveBeenCalledTimes(4);
  });

  it("caps backoff at 32000ms instead of widening forever", async () => {
    vi.useFakeTimers();
    const reconcile = vi.fn<() => Promise<boolean>>().mockResolvedValue(false);
    renderHook(() => useMissionControlPolling(reconcile));

    // 2000 + 4000 + 8000 + 16000 + 32000 = 62000ms to reach 5 ticks (interval caps at the 5th).
    await vi.advanceTimersByTimeAsync(62_000);
    expect(reconcile).toHaveBeenCalledTimes(5);

    // A 6th tick must be exactly 32000ms after the 5th, not wider -- proves the cap, not just "wide".
    await vi.advanceTimersByTimeAsync(31_999);
    expect(reconcile).toHaveBeenCalledTimes(5);
    await vi.advanceTimersByTimeAsync(1);
    expect(reconcile).toHaveBeenCalledTimes(6);
  });

  it("does not arm a new timer at all while hidden, and catches up immediately once visible again", async () => {
    vi.useFakeTimers();
    const visibility = vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    const reconcile = vi.fn<() => Promise<boolean>>().mockResolvedValue(false);
    const setTimeoutSpy = vi.spyOn(globalThis, "setTimeout");

    renderHook(() => useMissionControlPolling(reconcile));
    setTimeoutSpy.mockClear(); // isolate the assertion below to activity during the hidden window itself

    // Far past even the maximum 32000ms backoff -- if the timer merely skipped its own fetch (the
    // pre-16.16 behavior) rather than never being armed, this would still show up as scheduling
    // activity; a real stop schedules nothing at all.
    await vi.advanceTimersByTimeAsync(50_000);
    expect(reconcile).not.toHaveBeenCalled();
    expect(setTimeoutSpy).not.toHaveBeenCalled();

    visibility.mockReturnValue("visible");
    document.dispatchEvent(new Event("visibilitychange"));
    await vi.advanceTimersByTimeAsync(0);
    expect(reconcile).toHaveBeenCalledTimes(1);

    visibility.mockRestore();
    setTimeoutSpy.mockRestore();
  });

  it("resets backoff and reconciles immediately on a regained focus event", async () => {
    vi.useFakeTimers();
    const reconcile = vi.fn<() => Promise<boolean>>().mockResolvedValue(false);
    renderHook(() => useMissionControlPolling(reconcile));

    await vi.advanceTimersByTimeAsync(2_000);
    expect(reconcile).toHaveBeenCalledTimes(1); // backoff now pending at 4000ms

    window.dispatchEvent(new Event("focus"));
    await vi.advanceTimersByTimeAsync(0);
    expect(reconcile).toHaveBeenCalledTimes(2); // focus reconciled immediately, ahead of the 4000ms wait

    // Reset to the base cadence, not a continuation of the pre-focus backoff.
    await vi.advanceTimersByTimeAsync(2_000);
    expect(reconcile).toHaveBeenCalledTimes(3);
  });

  it("stops the timer on 'offline', and reconciles immediately with backoff reset on 'online'", async () => {
    vi.useFakeTimers();
    const reconcile = vi.fn<() => Promise<boolean>>().mockResolvedValue(false);
    renderHook(() => useMissionControlPolling(reconcile));

    await vi.advanceTimersByTimeAsync(2_000);
    expect(reconcile).toHaveBeenCalledTimes(1); // backoff now pending at 4000ms

    window.dispatchEvent(new Event("offline"));
    await vi.advanceTimersByTimeAsync(60_000); // well past even the 32000ms cap
    expect(reconcile).toHaveBeenCalledTimes(1); // the pending backoff timer was cleared, not just skipped

    window.dispatchEvent(new Event("online"));
    await vi.advanceTimersByTimeAsync(0);
    expect(reconcile).toHaveBeenCalledTimes(2); // reconnect reconciled immediately

    await vi.advanceTimersByTimeAsync(2_000);
    expect(reconcile).toHaveBeenCalledTimes(3); // resumed at the base cadence, not a leftover wide one
  });

  it("reconcileNow bypasses the visible/online gate for an explicit manual refresh, and resets backoff", async () => {
    vi.useFakeTimers();
    const visibility = vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    const reconcile = vi.fn<() => Promise<boolean>>().mockResolvedValue(false);
    const { result } = renderHook(() => useMissionControlPolling(reconcile));

    await vi.advanceTimersByTimeAsync(10_000);
    expect(reconcile).not.toHaveBeenCalled(); // hidden the whole time -- no automatic tick

    result.current.reconcileNow();
    await vi.advanceTimersByTimeAsync(0);
    expect(reconcile).toHaveBeenCalledTimes(1); // forced call went through despite still being hidden

    visibility.mockRestore();
  });

  it("keeps scheduling future ticks even when reconcile() itself rejects", async () => {
    vi.useFakeTimers();
    const reconcile = vi.fn<() => Promise<boolean>>()
      .mockRejectedValueOnce(new Error("boom"))
      .mockResolvedValue(true);
    renderHook(() => useMissionControlPolling(reconcile));

    await vi.advanceTimersByTimeAsync(2_000);
    expect(reconcile).toHaveBeenCalledTimes(1);

    // A rejection widens backoff the same as a no-op tick rather than derailing the schedule.
    await vi.advanceTimersByTimeAsync(2_000);
    expect(reconcile).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(2_000);
    expect(reconcile).toHaveBeenCalledTimes(2);
  });

  it("removes every listener and clears the pending timer on unmount", async () => {
    vi.useFakeTimers();
    const addWindowSpy = vi.spyOn(window, "addEventListener");
    const removeWindowSpy = vi.spyOn(window, "removeEventListener");
    const addDocSpy = vi.spyOn(document, "addEventListener");
    const removeDocSpy = vi.spyOn(document, "removeEventListener");
    const reconcile = vi.fn<() => Promise<boolean>>().mockResolvedValue(false);

    const { unmount } = renderHook(() => useMissionControlPolling(reconcile));
    unmount();

    for (const type of ["focus", "online", "offline"]) {
      const [, handler] = addWindowSpy.mock.calls.find(([eventType]) => eventType === type)!;
      expect(removeWindowSpy).toHaveBeenCalledWith(type, handler);
    }
    const [, visibilityHandler] = addDocSpy.mock.calls.find(([eventType]) => eventType === "visibilitychange")!;
    expect(removeDocSpy).toHaveBeenCalledWith("visibilitychange", visibilityHandler);

    await vi.advanceTimersByTimeAsync(60_000);
    expect(reconcile).not.toHaveBeenCalled();

    addWindowSpy.mockRestore(); removeWindowSpy.mockRestore();
    addDocSpy.mockRestore(); removeDocSpy.mockRestore();
  });
});
