import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../types/agent";
import type { MissionControlAction, MissionControlRunSummary } from "../types/mission-control";
import { MutationStatus } from "../ui/async/MutationStatus";
import type { MutationState } from "../ui/async/mutation-state";
import { resolveAgentDisplayName } from "./mission-control-labels";

export interface RunCardProps {
  agents: readonly AgentRegistryEntry[];
  mutation?: MutationState;
  onAct: (run: MissionControlRunSummary, action: MissionControlAction) => void;
  onDismissError: (run: MissionControlRunSummary) => void;
  onInspect: (run: MissionControlRunSummary) => void;
  run: MissionControlRunSummary;
}

/**
 * 16.7: prefers the Agent registry's own display name over the raw id, falling back to the raw id
 * (never blank, never the registry silently hiding a mismatch) when no entry matches -- see
 * mission-control-labels.ts's own doc comment for why a mismatch is a real, expected case and not
 * just defensive coding.
 *
 * When a run carries no `agentId` at all (an owner type the backend does not classify as Agent- or
 * generation-adjacent -- e.g. a bare "session", "loop_run", or the Web adapter's own "web_demo"
 * demo-seed owner), the raw `ownerType` itself is translated the same way `reasonCode` a few lines
 * below already is: a known-value lookup with a same-string fallback, never a raw internal token or
 * an untranslated i18n key leaking through.
 */
function runOwnerLabel(run: MissionControlRunSummary, agents: readonly AgentRegistryEntry[], t: (key: string, options?: Record<string, unknown>) => string): string {
  const agentLabel = resolveAgentDisplayName(agents, run.agentId);
  if (agentLabel) return agentLabel;
  return t(`missionControl.owner.${run.ownerType}`, { defaultValue: run.ownerType });
}

export function RunCard({ agents, mutation, onAct, onDismissError, onInspect, run }: RunCardProps) {
  const { t } = useTranslation();
  const pending = mutation?.pending ?? false;
  const ended = run.endedAt ?? run.updatedAt;
  const elapsed = Math.max(0, Date.parse(ended) - Date.parse(run.createdAt));
  return (
    <article className="rounded-md border border-border bg-card p-3" data-testid={`mission-run-${run.runId}`}>
      <button className="w-full text-left focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onClick={() => void onInspect(run)} type="button">
        <div className="flex flex-wrap items-center gap-2">
          <span className="min-w-0 flex-1 truncate text-sm font-medium">{run.title}</span>
          {run.runner ? (
            <span className="inline-flex max-w-44 items-center gap-1 rounded border border-primary/30 bg-primary/5 px-1.5 py-0.5 text-[11px] text-primary" data-runner={run.runner.kind}>
              <span>{t(`runner.kind.${run.runner.kind}`)}</span>
              {run.runner.hostLabel ? <span className="truncate text-muted-foreground">· {run.runner.hostLabel}</span> : null}
            </span>
          ) : null}
          <span className="rounded border border-border px-1.5 py-0.5 text-[11px]">{t(`missionControl.state.${run.state}`)}</span>
        </div>
        <p className="mt-1 text-xs text-muted-foreground">
          {runOwnerLabel(run, agents, t)} · {t("missionControl.elapsed", { seconds: Math.round(elapsed / 1000) })} · {t(`missionControl.verification.${run.verification}`)}
        </p>
        {run.reasonCode ? <p className="mt-1 text-xs text-warning">{t(`runner.reason.${run.reasonCode}`, { defaultValue: run.reasonCode })}</p> : null}
      </button>
      <div className="mt-2 flex flex-wrap items-center gap-2">
        <div className="flex flex-wrap gap-1">
          {run.actions.map((action) => (
            <button
              className="rounded-md border border-input px-2 py-1 text-xs hover:bg-muted focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
              data-action={action}
              disabled={pending}
              key={action}
              onClick={() => void onAct(run, action)}
              type="button"
            >
              {t(`missionControl.action.${action}`)}
            </button>
          ))}
        </div>
        <MutationStatus onDismiss={() => onDismissError(run)} state={mutation} />
      </div>
    </article>
  );
}
