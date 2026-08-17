export interface MissionControlUpdate { runId: string; version: number; kind: "progress" | "usage" | "state" | "attention" | "terminal" }

export function createMissionControlCoalescer(flush: (updates: MissionControlUpdate[]) => void, delay = 250) {
  const pending = new Map<string, MissionControlUpdate>();
  let timer: ReturnType<typeof setTimeout> | null = null;
  const drain = () => {
    if (timer) clearTimeout(timer);
    timer = null;
    const updates = [...pending.values()]; pending.clear();
    if (updates.length) flush(updates);
  };
  return {
    push(update: MissionControlUpdate) {
      const current = pending.get(update.runId);
      if (current && current.version > update.version) return;
      pending.set(update.runId, update);
      if (["state", "attention", "terminal"].includes(update.kind)) { drain(); return; }
      if (!timer) timer = setTimeout(drain, delay);
    },
    dispose() { if (timer) clearTimeout(timer); timer = null; pending.clear(); },
    flush: drain,
  };
}
