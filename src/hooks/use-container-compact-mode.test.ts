// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useContainerCompactMode } from "./use-container-compact-mode";

/** Captures the callback each `new ResizeObserver(callback)` call is constructed with, so a test
 *  can fire it manually with a fake `contentRect.width` -- jsdom itself never triggers a real
 *  resize (src/test/setup.ts's own no-op stub is why every *other* consumer of this mechanism only
 *  ever asserts the non-compact default). This is the first test to exercise the callback firing. */
function installControllableResizeObserver() {
  let latestCallback: ResizeObserverCallback | null = null;
  class ControllableResizeObserver {
    constructor(callback: ResizeObserverCallback) {
      latestCallback = callback;
    }
    observe() {
      /* no-op: this stub fires only when the test calls fire() explicitly */
    }
    unobserve() {
      /* no-op */
    }
    disconnect() {
      /* no-op */
    }
  }
  vi.stubGlobal("ResizeObserver", ControllableResizeObserver as unknown as typeof ResizeObserver);
  return {
    fire(width: number) {
      act(() => {
        latestCallback?.([{ contentRect: { width } } as ResizeObserverEntry], {} as ResizeObserver);
      });
    },
  };
}

describe("useContainerCompactMode", () => {
  beforeEach(() => vi.unstubAllGlobals());
  afterEach(() => vi.unstubAllGlobals());

  it("starts non-compact before any measurement arrives", () => {
    installControllableResizeObserver();
    const container = document.createElement("div");
    const { result } = renderHook(() => useContainerCompactMode({ current: container }, 640));
    expect(result.current).toBe(false);
  });

  it("flips to compact once the observed width drops below the threshold", () => {
    const resizeObserver = installControllableResizeObserver();
    const container = document.createElement("div");
    const { result } = renderHook(() => useContainerCompactMode({ current: container }, 640));

    resizeObserver.fire(480);
    expect(result.current).toBe(true);
  });

  it("stays non-compact when the observed width is at or above the threshold", () => {
    const resizeObserver = installControllableResizeObserver();
    const container = document.createElement("div");
    const { result } = renderHook(() => useContainerCompactMode({ current: container }, 640));

    resizeObserver.fire(640);
    expect(result.current).toBe(false);

    resizeObserver.fire(900);
    expect(result.current).toBe(false);
  });

  it("respects a caller-specific threshold rather than a shared hardcoded value", () => {
    const resizeObserver = installControllableResizeObserver();
    const container = document.createElement("div");
    const { result } = renderHook(() => useContainerCompactMode({ current: container }, 1280));

    resizeObserver.fire(1000);
    expect(result.current).toBe(true);
  });

  it("does nothing when the container ref has no current element yet", () => {
    installControllableResizeObserver();
    const { result } = renderHook(() => useContainerCompactMode({ current: null }, 640));
    expect(result.current).toBe(false);
  });
});
