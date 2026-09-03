import { TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../types/agent";
import { scheduledTaskAgentCapability } from "./scheduled-task-agent-capability";
import { ScheduledTaskExecutionNotice } from "./scheduled-task-execution-notice";

export interface ScheduledTaskCapabilityNoticeProps {
  agent: AgentRegistryEntry | undefined;
  agentId: string;
}

/**
 * 19.6's "capability notice" and 19.15's "capability-driven help" are treated as the same section
 * rather than two separate ones: both ask for an honest statement of what this task can actually
 * do, given the real system, next to the rest of the detail view's facts. It combines:
 *  - the Agent availability check (`scheduledTaskAgentCapability`), warning-styled the same way
 *    `EvaluationReviewStep`'s own flagged-Agent box already does (`TriangleAlert` + the warning
 *    token), shown only when there is something to flag; and
 *  - `ScheduledTaskExecutionNotice`, always shown -- the timezone/DST and app-open/catch-up facts
 *    are true regardless of whether the Agent itself is currently available.
 */
export function ScheduledTaskCapabilityNotice({ agent, agentId }: ScheduledTaskCapabilityNoticeProps) {
  const { t } = useTranslation();
  const capability = scheduledTaskAgentCapability(agent);
  const message = !agent
    ? t("scheduledTasks.capability.agentMissing", { agentId })
    : (agent.unavailableReason ?? t(`scheduledTasks.agentStatus.${agent.availabilityState}`));

  return (
    <div className="grid gap-2 rounded-md border border-border p-3" data-testid="scheduled-task-capability-notice">
      <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("scheduledTasks.capability.title")}</h4>
      {capability ? (
        <p className="flex items-start gap-1.5 rounded-md border border-[hsl(var(--warning))]/40 bg-[hsl(var(--warning))]/5 p-2 text-xs text-[hsl(var(--warning))]" role="status">
          <TriangleAlert aria-hidden="true" className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          <span>{message}</span>
        </p>
      ) : null}
      <ScheduledTaskExecutionNotice />
    </div>
  );
}
