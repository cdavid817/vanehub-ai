import { useEffect, useState } from "react";
import { agentService } from "../../services/runtime-agent-client";
import type { AgentRun } from "../../types/agent-run";
import { AgentRunStatus } from "./agent-run-status";

export function AgentRunOwnerStatus({ ownerId, ownerType }: {
  ownerId: string;
  ownerType: string;
}) {
  const [run, setRun] = useState<AgentRun | null>(null);

  useEffect(() => {
    let active = true;
    const refresh = () => void agentService.listAgentRuns(0, 1, { ownerId, ownerType })
      .then((page) => { if (active) setRun(page.items[0] ?? null); })
      .catch(() => { if (active) setRun(null); });
    refresh();
    const timer = window.setInterval(refresh, 1_000);
    return () => { active = false; window.clearInterval(timer); };
  }, [ownerId, ownerType]);

  if (!run) return null;
  const elapsedMs = Math.max(0, Date.parse(run.updatedAt) - Date.parse(run.createdAt));
  const elapsed = `${Math.floor(elapsedMs / 60_000)}:${String(Math.floor(elapsedMs / 1_000) % 60).padStart(2, "0")}`;
  return (
    <AgentRunStatus
      elapsed={elapsed}
      onCancel={() => void agentService.cancelAgentRun(run.id, run.version).then(setRun)}
      onResume={() => void agentService.resumeAgentRun(run.id, run.version).then(setRun)}
      run={run}
    />
  );
}
