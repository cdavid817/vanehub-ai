import { useCallback, useEffect, useRef } from "react";

const BASE_INTERVAL_MS = 2_000;
const MAX_INTERVAL_MS = 32_000;

export interface UseMissionControlPollingResult {
  /**
   * Runs `reconcile()` immediately, ignoring the visible/online gate the automatic triggers below
   * respect, and resets backoff to `BASE_INTERVAL_MS` -- the same reset every automatic trigger
   * applies. Wired to the Toolbar's own Refresh button: a deliberate user click should always
   * attempt the fetch (and surface its real error if genuinely offline) rather than silently no-op
   * just because the automatic gate would have skipped it.
   */
  reconcileNow: () => void;
}

/**
 * Task 16.16: replaces the previous flat `window.setInterval(reconcile, 2_000)`
 * (mission-control.tsx, pre-16.16) -- which kept ticking every 2s even while the tab stayed hidden,
 * only skipping the `load()` call itself inside the callback -- with three real behaviors, none of
 * which existed before:
 *
 * 1. The timer stops while hidden/offline, not just the fetch inside it: `scheduleNext` only arms a
 *    `setTimeout` when `document.visibilityState === "visible"` AND `navigator.onLine`; otherwise no
 *    timer exists at all until a real trigger (see 3) re-arms one, instead of a callback firing into
 *    a no-op guard every 2s for as long as a tab stays backgrounded.
 * 2. Real bounded backoff: an interval whose `reconcile()` reports no meaningful change doubles
 *    (`BASE_INTERVAL_MS` -> ... -> `MAX_INTERVAL_MS`, capped at 16x base) instead of repeating the
 *    same 2s cadence forever against a Run list that is not changing. The moment `reconcile()`
 *    reports a real change, or any user-observable trigger fires (focus, visibility regained, a
 *    network reconnect, or the exposed `reconcileNow`), the interval resets to `BASE_INTERVAL_MS`.
 * 3. `online`/`offline` listeners, alongside the pre-existing `focus`/`visibilitychange`: a genuine
 *    network reconnect now reconciles immediately the same way regaining focus already did, and
 *    going offline stops the timer for the same reason going hidden does -- a fetch already known to
 *    fail is not worth scheduling.
 *
 * True backend-pushed "coalesced events" remain out of scope here, confirmed (not assumed) by
 * grepping every `.emit(`/`app_handle.emit` call under `src-tauri/src/contexts/operations/` (Mission
 * Control's own Rust context) and `bootstrap/agent_run_controls.rs` (the action-command path
 * 16.14/16.15 already document): the only two real events in `operations` are
 * `SESSION_LOG_EVENT`/`SESSION_LOG_REPAIR_EVENT` (`log_index_support.rs`), nothing for `AgentRun`
 * state changes, and the action-command path emits nothing at all. `event-coalescer.ts` (added years
 * earlier, alongside the original Mission Control in #159) already implements the client-side
 * batching half of "coalesced events" -- immediate flush for `state`/`attention`/`terminal` update
 * kinds, debounced batching for `progress`/`usage` -- but has no real producer feeding it a
 * `MissionControlUpdate` anywhere in this codebase (confirmed: grepping `createMissionControlCoalescer`
 * turns up only its own declaration and its own test), because there is no backend event stream yet
 * for it to coalesce from. That is the same class of gap task 18.14 already documented for
 * Evaluation's identical ask -- new cross-cutting backend work, not a frontend-only wiring pass. This
 * hook only replaces the polling half with the reconciliation discipline that is achievable today.
 */
export function useMissionControlPolling(reconcile: () => Promise<boolean>): UseMissionControlPollingResult {
  const triggerRef = useRef<() => void>(() => {});

  useEffect(() => {
    let disposed = false;
    let inFlight = false;
    let timer: ReturnType<typeof setTimeout> | null = null;
    let intervalMs = BASE_INTERVAL_MS;

    const canPoll = () => document.visibilityState === "visible" && navigator.onLine;
    const clearTimer = () => { if (timer !== null) { clearTimeout(timer); timer = null; } };

    const scheduleNext = () => {
      clearTimer();
      if (disposed || !canPoll()) return;
      timer = setTimeout(() => void run(false, false), intervalMs);
    };

    // `forced` bypasses the visible/online gate (only the exposed `reconcileNow` needs this, for a
    // deliberate manual refresh click). `reset` marks a user-observable trigger (focus, regained
    // visibility, a network reconnect, or the manual refresh) -- the next interval always lands back
    // on `BASE_INTERVAL_MS` for these regardless of what this particular fetch itself reports, since
    // "the reader is plausibly looking again" outweighs "this one fetch happened to see no change".
    // Only a natural backoff-timer tick (`scheduleNext`'s own callback, `reset === false`) actually
    // widens on a no-op result.
    const run = async (reset: boolean, forced: boolean) => {
      if (disposed || inFlight) return;
      if (!forced && !canPoll()) return;
      inFlight = true;
      clearTimer();
      try {
        const changed = await reconcile();
        if (!disposed) intervalMs = (reset || changed) ? BASE_INTERVAL_MS : Math.min(intervalMs * 2, MAX_INTERVAL_MS);
      } catch {
        // reconcile() is expected to swallow its own errors (mission-control.tsx's load() always
        // has; matches loop-run-polling.ts's identical contract) -- this only guards a future
        // caller that does not, so one throwing reconcile cannot permanently kill the schedule loop.
        if (!disposed) intervalMs = reset ? BASE_INTERVAL_MS : Math.min(intervalMs * 2, MAX_INTERVAL_MS);
      } finally {
        inFlight = false;
        if (!disposed) scheduleNext();
      }
    };

    // Shared by focus/visibilitychange-to-visible/online: all three are "a moment the reader is
    // plausibly looking again," gated by the same visible+online check `scheduleNext` uses, so a
    // stray `focus` while still hidden, or an `online` event while still backgrounded, is a no-op
    // rather than an unwanted fetch.
    const reconcileFromEvent = () => void run(true, false);
    triggerRef.current = () => void run(true, true);

    const onVisibility = () => { if (document.visibilityState === "visible") reconcileFromEvent(); else clearTimer(); };
    const onOffline = () => clearTimer();

    scheduleNext();
    window.addEventListener("focus", reconcileFromEvent);
    document.addEventListener("visibilitychange", onVisibility);
    window.addEventListener("online", reconcileFromEvent);
    window.addEventListener("offline", onOffline);

    return () => {
      disposed = true;
      clearTimer();
      triggerRef.current = () => {};
      window.removeEventListener("focus", reconcileFromEvent);
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("online", reconcileFromEvent);
      window.removeEventListener("offline", onOffline);
    };
  }, [reconcile]);

  return { reconcileNow: useCallback(() => triggerRef.current(), []) };
}
