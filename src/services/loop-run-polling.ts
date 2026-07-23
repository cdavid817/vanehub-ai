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
    if (!active || polling) return;
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
  return () => {
    active = false;
    clearInterval(timer);
  };
}
