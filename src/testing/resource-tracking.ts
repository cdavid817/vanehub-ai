import { vi } from "vitest";

/**
 * Task 21.15: a small, shared "how many X are alive right now" toolkit, so proving a destination's
 * teardown genuinely releases its page-owned resources (21.16) does not mean every domain
 * reinvents its own counting scheme.
 *
 * Verified before building: this codebase already has several *ad hoc* proxies for "did the
 * resource go away", but nothing that answers "how many are live right now" directly, and nothing
 * shared across domains --
 * - `use-mission-control-polling.test.ts` and `loop-run-polling.test.ts` both spy on
 *   `add`/`removeEventListener` and assert the same handler reference was removed.
 * - `evaluation-center.test.tsx`'s "stops polling once unmounted" advances a real 1.5s wall clock
 *   and asserts the tracked mock was not called again.
 * Both only prove *no further activity*, never *the underlying handle/observer was actually
 * deallocated*, and neither shape generalizes to `ResizeObserver` or to ad hoc
 * acquire/release-style resources at all. This module covers three resource shapes instead:
 *
 * - Timers (`setInterval`/`setTimeout`): `getActivePendingTimerCount` wraps Vitest's own
 *   fake-timer engine (`vi.getTimerCount()`), which already tracks exactly this count -- no
 *   reimplementation needed, just a guarded, documented entry point.
 * - `ResizeObserver`: `createTrackedResizeObserver`. No fake-timer equivalent exists for this, and
 *   jsdom implements no real one either (`src/test/setup.ts`'s own global no-op stub) -- six call
 *   sites across the test suite (grepped) each hand-roll a throwaway no-op or "controllable" stub
 *   class today, and none of them count. Install this one with
 *   `vi.stubGlobal("ResizeObserver", tracked.Ctor)`.
 * - Anything acquired and released rather than fired on a clock (a subscription's own unsubscribe,
 *   a "heavy panel" mount/unmount pair): `createResourceRegistry`, a bare counter a test double
 *   calls into directly, for cases the two mechanisms above do not shape-match.
 */

/**
 * The number of timers (`setInterval` or `setTimeout`) Vitest's fake-timer engine is still holding
 * -- i.e. armed but not yet fired, and not yet cleared. Requires `vi.useFakeTimers()`: against real
 * timers there is nothing deterministic to read, so this throws rather than silently returning 0
 * (which would look identical to "genuinely released").
 */
export function getActivePendingTimerCount(): number {
  if (!vi.isFakeTimers()) {
    throw new Error(
      "getActivePendingTimerCount() requires vi.useFakeTimers() first -- it reads vitest's own " +
        "fake-timer queue, which does not exist against real timers.",
    );
  }
  return vi.getTimerCount();
}

export interface TrackedResizeObserver {
  /** Install via `vi.stubGlobal("ResizeObserver", tracked.Ctor)`. */
  Ctor: typeof ResizeObserver;
  /** Observers that have called `observe()` without a matching `disconnect()` yet. */
  activeCount: () => number;
  /** Invokes the most recently constructed observer's callback with a fake `contentRect.width`,
   *  standing in for jsdom's own real-but-nonexistent resize delivery. */
  fire: (width: number) => void;
}

/**
 * Counts by `observe()`/`disconnect()`, not by construction: every real consumer in this codebase
 * (`useContainerCompactMode`) constructs exactly one observer and calls `observe()` exactly once
 * before its effect's `disconnect()` cleanup, so this granularity matches actual usage without
 * needing to track which specific element each call targeted. `unobserve()` is intentionally a
 * no-op for the same reason `ControllableResizeObserver` (use-container-compact-mode.test.ts's own
 * prior local stub) already left it one: no consumer here calls it.
 */
export function createTrackedResizeObserver(): TrackedResizeObserver {
  let active = 0;
  let latestCallback: ResizeObserverCallback | null = null;

  class TrackedResizeObserverCtor {
    constructor(callback: ResizeObserverCallback) {
      latestCallback = callback;
    }
    observe() {
      active += 1;
    }
    unobserve() {
      /* no-op: no consumer in this codebase calls it */
    }
    disconnect() {
      if (active > 0) active -= 1;
    }
  }

  return {
    Ctor: TrackedResizeObserverCtor as unknown as typeof ResizeObserver,
    activeCount: () => active,
    fire: (width: number) => {
      latestCallback?.([{ contentRect: { width } } as ResizeObserverEntry], {} as ResizeObserver);
    },
  };
}

export interface ResourceRegistry {
  /** Returns an id to hand back to `release`. */
  acquire: () => number;
  release: (id: number) => void;
  activeCount: () => number;
}

/**
 * A bare acquire/release counter for resources that do not fit the timer or observer shape --
 * e.g. a fake `subscribeMessageEvents` calling `acquire()` on subscribe and `release()` on its
 * returned unsubscribe, or a lazily-mounted "heavy panel" test double calling `acquire()` on mount
 * and `release()` on unmount. Deliberately untyped beyond the id: callers that need to assert
 * *which* resource is still live, not just how many, should track that themselves alongside the id
 * this returns.
 */
export function createResourceRegistry(): ResourceRegistry {
  const live = new Set<number>();
  let nextId = 0;
  return {
    acquire: () => {
      nextId += 1;
      live.add(nextId);
      return nextId;
    },
    release: (id: number) => {
      live.delete(id);
    },
    activeCount: () => live.size,
  };
}
