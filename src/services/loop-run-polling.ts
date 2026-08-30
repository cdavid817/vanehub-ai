import type { LoopEvent, LoopRun } from "../types/loop";

const LOOP_POLL_INTERVAL_MS = 1_000;

export function subscribeLoopRunPolling(
  loadRun: () => Promise<LoopRun>,
  handler: (event: LoopEvent) => void,
  intervalMs = LOOP_POLL_INTERVAL_MS,
): () => void {
  let active = true;
  let polling = false;
  let fingerprint: string | null = null;

  const poll = async () => {
    // design.md Decision 6: hidden pages must not retain high-frequency polling — a backgrounded
    // tab/window skips the fetch here rather than unsubscribing, matching Mission Control's
    // existing reconcile guard (mission-control.tsx) so a regained-focus tick still catches up fast.
    if (!active || polling || document.visibilityState !== "visible") return;
    polling = true;
    try {
      const run = await loadRun();
      if (!active) return;
      const nextFingerprint = JSON.stringify(run);
      if (fingerprint !== null && nextFingerprint !== fingerprint) {
        handler({ kind: "run-updated", run });
      }
      fingerprint = nextFingerprint;
    } catch {
      // A transient native read failure must not terminate future refreshes.
    } finally {
      polling = false;
    }
  };

  void poll();
  const timer = setInterval(() => void poll(), intervalMs);
  const reconcileOnReturn = () => { if (document.visibilityState === "visible") void poll(); };
  window.addEventListener("focus", reconcileOnReturn);
  document.addEventListener("visibilitychange", reconcileOnReturn);
  return () => {
    active = false;
    clearInterval(timer);
    window.removeEventListener("focus", reconcileOnReturn);
    document.removeEventListener("visibilitychange", reconcileOnReturn);
  };
}
