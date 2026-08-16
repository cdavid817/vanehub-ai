import { useTranslation } from "react-i18next";
import type { AgentRun } from "../../types/agent-run";
import { Badge } from "./badge";
import { Button } from "./button";

interface AgentRunStatusProps {
  run: AgentRun;
  elapsed: string;
  onCancel?: () => void;
  onResume?: () => void;
}

export function AgentRunStatus({ run, elapsed, onCancel, onResume }: AgentRunStatusProps) {
  const { t } = useTranslation();
  const terminal = ["completed", "failed", "cancelled"].includes(run.state);
  const resumable = ["paused", "blocked", "stuck"].includes(run.state);
  return (
    <div aria-live="polite" className="flex min-w-0 flex-nowrap items-center gap-2 overflow-x-auto whitespace-nowrap text-xs text-muted-foreground" data-reason-code={run.reasonCode ?? undefined} data-state={run.state} data-testid="agent-run-status" role="status">
      <Badge tone="muted">{t(`run.status.${run.state}`)}</Badge>
      <span>{t("run.elapsed", { elapsed })}</span>
      {run.reasonCode ? <span className="truncate">{t("run.reason", { reason: run.reasonCode })}</span> : null}
      {run.retryCount > 0 ? <span>{t("run.retry", { count: run.retryCount })}</span> : null}
      {!terminal && onCancel ? <Button onClick={onCancel} size="sm" type="button" variant="outline">{t("run.cancel")}</Button> : null}
      {resumable && onResume ? <Button onClick={onResume} size="sm" type="button" variant="outline">{t("run.resume")}</Button> : null}
    </div>
  );
}
