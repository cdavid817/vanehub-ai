import type { PlanRunDetail } from "../types/plan";

export function subscribePlanRunPolling(
  loadRun: () => Promise<PlanRunDetail>,
  handler: (run: PlanRunDetail) => void,
  intervalMs = 1_000,
): () => void {
  let active = true;
  let loading = false;
  let fingerprint: string | null = null;
  const poll = async () => {
    if (!active || loading) return;
    loading = true;
    try {
      const run = await loadRun();
      if (!active) return;
      const next = `${run.status}:${run.updatedAt}:${run.completedTasks}`;
      if (fingerprint !== null && next !== fingerprint) handler(run);
      fingerprint = next;
    } catch {
      // Transient reads do not stop later bounded refreshes.
    } finally {
      loading = false;
    }
  };
  void poll();
  const timer = setInterval(() => void poll(), intervalMs);
  return () => { active = false; clearInterval(timer); };
}
