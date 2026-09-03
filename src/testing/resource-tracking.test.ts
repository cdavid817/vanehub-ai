// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createResourceRegistry,
  createTrackedResizeObserver,
  getActivePendingTimerCount,
} from "./resource-tracking";

describe("getActivePendingTimerCount", () => {
  afterEach(() => vi.useRealTimers());

  it("throws instead of silently returning 0 when fake timers are not installed", () => {
    expect(() => getActivePendingTimerCount()).toThrow(/vi\.useFakeTimers/);
  });

  it("reflects setInterval/setTimeout arming and clearing", () => {
    vi.useFakeTimers();
    expect(getActivePendingTimerCount()).toBe(0);

    const timeout = setTimeout(() => {}, 1_000);
    const interval = setInterval(() => {}, 1_000);
    expect(getActivePendingTimerCount()).toBe(2);

    clearTimeout(timeout);
    expect(getActivePendingTimerCount()).toBe(1);

    clearInterval(interval);
    expect(getActivePendingTimerCount()).toBe(0);
  });

  it("drops a fired one-shot timeout from the count without an explicit clear", async () => {
    vi.useFakeTimers();
    setTimeout(() => {}, 1_000);
    expect(getActivePendingTimerCount()).toBe(1);

    await vi.advanceTimersByTimeAsync(1_000);
    expect(getActivePendingTimerCount()).toBe(0);
  });
});

describe("createTrackedResizeObserver", () => {
  it("counts observe() calls and decrements on disconnect()", () => {
    const tracked = createTrackedResizeObserver();
    const Ctor = tracked.Ctor;
    expect(tracked.activeCount()).toBe(0);

    const first = new Ctor(() => {});
    first.observe(document.createElement("div"));
    expect(tracked.activeCount()).toBe(1);

    const second = new Ctor(() => {});
    second.observe(document.createElement("div"));
    expect(tracked.activeCount()).toBe(2);

    first.disconnect();
    expect(tracked.activeCount()).toBe(1);

    second.disconnect();
    expect(tracked.activeCount()).toBe(0);
  });

  it("never goes negative on an extra disconnect()", () => {
    const tracked = createTrackedResizeObserver();
    const observer = new tracked.Ctor(() => {});
    observer.observe(document.createElement("div"));
    observer.disconnect();
    observer.disconnect();
    expect(tracked.activeCount()).toBe(0);
  });

  it("fire() invokes the most recently constructed observer's callback", () => {
    const tracked = createTrackedResizeObserver();
    const first = vi.fn();
    const second = vi.fn();
    new tracked.Ctor(first).observe(document.createElement("div"));
    new tracked.Ctor(second).observe(document.createElement("div"));

    tracked.fire(480);
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledWith(
      [{ contentRect: { width: 480 } }],
      {},
    );
  });
});

describe("createResourceRegistry", () => {
  it("tracks acquire/release as a live count, and issues distinct ids", () => {
    const registry = createResourceRegistry();
    expect(registry.activeCount()).toBe(0);

    const first = registry.acquire();
    const second = registry.acquire();
    expect(first).not.toBe(second);
    expect(registry.activeCount()).toBe(2);

    registry.release(first);
    expect(registry.activeCount()).toBe(1);

    registry.release(second);
    expect(registry.activeCount()).toBe(0);
  });

  it("ignores a release() for an id that was already released", () => {
    const registry = createResourceRegistry();
    const id = registry.acquire();
    registry.release(id);
    registry.release(id);
    expect(registry.activeCount()).toBe(0);
  });
});
