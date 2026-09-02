import { AlertTriangle } from "lucide-react";
import { type RefObject } from "react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../types/agent";
import type { MissionControlAction, MissionControlOverview, MissionControlRunSummary } from "../types/mission-control";
import type { MutationState } from "../ui/async/mutation-state";
import { RunCard } from "./mission-control-run-card";

export interface RunSectionProps {
  agents: readonly AgentRegistryEntry[];
  mutations: ReadonlyMap<string, MutationState>;
  onAct: (run: MissionControlRunSummary, action: MissionControlAction) => void;
  onDismissError: (run: MissionControlRunSummary) => void;
  onInspect: (run: MissionControlRunSummary) => void;
  runs: MissionControlRunSummary[];
  title: string;
  urgent?: boolean;
}

/** 16.3's own "Run collection": one labeled group of `RunCard`s (Attention/Active/Recent) --
 *  unchanged from the original inline version beyond threading `agents` through for 16.7. */
export function RunSection({ agents, mutations, onAct, onDismissError, onInspect, runs, title, urgent = false }: RunSectionProps) {
  if (!runs.length) return null;
  return (
    <section className="mb-4">
      <h2 className="mb-2 flex items-center gap-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {urgent ? <AlertTriangle aria-hidden="true" className="h-3.5 w-3.5 text-warning" /> : null}
        {title}
      </h2>
      <div className="grid gap-2">
        {runs.map((run) => (
          <RunCard agents={agents} key={run.runId} mutation={mutations.get(run.runId)} onAct={onAct} onDismissError={onDismissError} onInspect={onInspect} run={run} />
        ))}
      </div>
    </section>
  );
}

export interface MissionControlRunListProps {
  agents: readonly AgentRegistryEntry[];
  listRef: RefObject<HTMLDivElement | null>;
  loading: boolean;
  mutations: ReadonlyMap<string, MutationState>;
  onAct: (run: MissionControlRunSummary, action: MissionControlAction) => void;
  onDismissError: (run: MissionControlRunSummary) => void;
  onInspect: (run: MissionControlRunSummary) => void;
  onNextPage: (cursor: string) => void;
  onScroll: (scrollTop: number) => void;
  overview: MissionControlOverview | null;
}

/** The scrollable left-hand region: Attention/Active/Recent sections, pagination, and the explicit
 *  empty state -- split out of mission-control.tsx unchanged beyond threading `agents` through. */
export function MissionControlRunList({ agents, listRef, loading, mutations, onAct, onDismissError, onInspect, onNextPage, onScroll, overview }: MissionControlRunListProps) {
  const { t } = useTranslation();
  const nextCursor = overview ? (overview.attention.nextCursor ?? overview.active.nextCursor ?? overview.recent.nextCursor) : null;
  const empty = Boolean(!loading && overview
    && overview.attention.items.length + overview.active.items.length + overview.recent.items.length === 0);
  return (
    <div className="min-h-0 overflow-y-auto p-3" onScroll={(event) => onScroll(event.currentTarget.scrollTop)} ref={listRef}>
      <RunSection agents={agents} mutations={mutations} onAct={onAct} onDismissError={onDismissError} onInspect={onInspect} runs={overview?.attention.items ?? []} title={t("missionControl.attention")} urgent />
      <RunSection agents={agents} mutations={mutations} onAct={onAct} onDismissError={onDismissError} onInspect={onInspect} runs={overview?.active.items ?? []} title={t("missionControl.active")} />
      <RunSection agents={agents} mutations={mutations} onAct={onAct} onDismissError={onDismissError} onInspect={onInspect} runs={overview?.recent.items ?? []} title={t("missionControl.recent")} />
      {nextCursor ? <button className="rounded-md border border-input px-3 py-1.5 text-xs" onClick={() => onNextPage(nextCursor)} type="button">{t("missionControl.nextPage")}</button> : null}
      {empty ? <p className="p-8 text-center text-sm text-muted-foreground">{t("missionControl.empty")}</p> : null}
    </div>
  );
}
