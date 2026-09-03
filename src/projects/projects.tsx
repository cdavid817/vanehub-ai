import { RefreshCw } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { AsyncBoundary } from "../ui/async/AsyncBoundary";
import type { AsyncViewState } from "../ui/async/async-view-state";
import { EmptyState } from "../ui/empty-state/EmptyState";
import { useProjectWorkspaces } from "./use-project-workspaces";
import { useWorkspaceReconnect } from "./use-workspace-reconnect";
import { selectWorkspaceView, workspaceViews, type WorkspaceView } from "./workspace-filter";
import { WorkspaceCard } from "./workspace-card";
import { WorkspaceDetail } from "./workspace-detail";
import type { WorkspaceSummary } from "./workspace-summary";

interface WorkspaceViewListProps {
  view: WorkspaceView;
  workspaces: WorkspaceSummary[];
  selectedWorkspaceId: string | null;
  onSelect: (workspaceId: string) => void;
}

/**
 * Renders `workspaces` for the active view. Split out from `Projects` so the "only 'unavailable'
 * can be filtered-empty while the raw list is not" reasoning (see workspace-filter.ts) has one
 * place to live instead of being re-derived at each call site. Single-column now (13.7): the list
 * lives beside `WorkspaceDetail` in a master-detail split, matching `GoalCenter`'s own list column
 * width rather than the old multi-column grid this page used before it had a detail pane to make
 * room for.
 */
function WorkspaceViewList({ onSelect, selectedWorkspaceId, view, workspaces }: WorkspaceViewListProps) {
  const { t } = useTranslation();
  const visible = selectWorkspaceView(workspaces, view);
  if (visible.length === 0) {
    return (
      <EmptyState
        description={t("projects.view.unavailableEmpty.description")}
        title={t("projects.view.unavailableEmpty.title")}
        variant="no-filter-match"
      />
    );
  }
  return (
    <ul aria-label={t("projects.listLabel")} className="grid content-start gap-2">
      {visible.map((workspace) => (
        <li key={workspace.workspaceId}>
          <WorkspaceCard
            onSelect={() => onSelect(workspace.workspaceId)}
            selected={workspace.workspaceId === selectedWorkspaceId}
            workspace={workspace}
          />
        </li>
      ))}
    </ul>
  );
}

/**
 * Read-only aggregation entry point for §13 (design.md Decision 18). Real content replacing the
 * former placeholder: local project history, SSH known workspaces, SSH connection trust, and
 * recent sessions, joined client-side by `useProjectWorkspaces` — no new Tauri command or
 * writable cross-domain table. Scoped to 13.1/13.5/13.6/13.11, a partial 13.4 (recent/all/
 * unavailable only), and 13.7's own read-only master-detail split (list + `WorkspaceDetail`,
 * mirroring `GoalCenter`'s established layout); state-aware actions and the remaining views are
 * follow-up work — see this increment's own report for the full list of deferred tasks.
 *
 * Selection is local component state, not route-backed, even though `WorkbenchLocation`'s own
 * `projects.projectId` route slot already exists (`workbench-route.ts`) and `projects-destination.tsx`
 * flags it as this task's own to wire up: `PlanDestination`'s identical situation
 * (`PlanSection.goalId`/`workItemId`, never consumed by `GoalCenter`/`WorkBoard` either) is already
 * on record in this exact codebase as "content work for a later milestone," not an oversight —
 * `plan-destination.tsx`'s own comment says so directly. Route-backed restoration across
 * navigation/reload is what 13.12's own "list-then-detail composition, restore filters and scroll
 * anchor on Back" already owns; wiring `projectId` here now would duplicate that decision ahead of
 * it rather than follow the same precedent this codebase already chose for Goals.
 *
 * Task 13.8's three cross-cutting actions (Continue Session, New Session, Settings) are forwarded
 * straight through from `projects-destination.tsx` -- this component has no reason to know about
 * `goToSessions`/`SettingsPageId`. Reconnect is the one action this component itself owns end to
 * end (`use-workspace-reconnect.ts`), the same split `goal-center.tsx`/`use-goal-center-actions.ts`
 * already establish: mutations that belong to *this* list's own selection live here, navigation
 * that leaves this destination entirely is somebody else's to own.
 */
export function Projects({ onContinueSession, onNewSession, onOpenSshSettings }: {
  onContinueSession: (sessionId: string) => void;
  onNewSession: (workspace: Pick<WorkspaceSummary, "workspaceId" | "kind">) => void;
  onOpenSshSettings: () => void;
}) {
  const { t } = useTranslation();
  const { data, error, loading, reload } = useProjectWorkspaces();
  const [view, setView] = useState<WorkspaceView>("recent");
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string | null>(null);
  const { mutations: reconnectMutations, reconnect } = useWorkspaceReconnect(() => void reload());

  const asyncState: AsyncViewState<WorkspaceSummary[]> = {
    data,
    error: error ? { kind: "error", message: error, retryable: true } : undefined,
    initialLoading: loading,
    refreshing: loading && data !== undefined,
    stale: false,
  };

  return (
    <section aria-labelledby="projects-title" className="ucd-panel flex h-full min-h-0 flex-1 flex-col overflow-hidden rounded-lg" id="projects">
      <header className="grid shrink-0 gap-3 border-b border-border p-3 md:p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h1 className="text-base font-semibold" id="projects-title">{t("projects.title")}</h1>
            <p className="text-xs text-muted-foreground">{t("projects.subtitle")}</p>
          </div>
          <Button disabled={loading} onClick={() => void reload()} size="sm" type="button" variant="outline">
            <RefreshCw aria-hidden="true" className="h-4 w-4" />
            {t("projects.refresh")}
          </Button>
        </div>
        <div aria-label={t("projects.viewLabel")} className="flex gap-1" role="tablist">
          {workspaceViews.map((candidate) => (
            <Button
              aria-selected={view === candidate}
              key={candidate}
              onClick={() => setView(candidate)}
              role="tab"
              size="sm"
              type="button"
              variant={view === candidate ? "default" : "outline"}
            >
              {t(`projects.view.${candidate}`)}
            </Button>
          ))}
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto p-3">
        <AsyncBoundary
          emptyState={{ description: t("projects.empty.description"), title: t("projects.empty.title") }}
          isEmpty={(list) => list.length === 0}
          onRetry={() => void reload()}
          state={asyncState}
        >
          {(list) => {
            const selected = list.find((workspace) => workspace.workspaceId === selectedWorkspaceId) ?? null;
            return (
              <div className="grid gap-3 md:grid-cols-[minmax(16rem,22rem)_1fr]">
                <WorkspaceViewList
                  onSelect={setSelectedWorkspaceId}
                  selectedWorkspaceId={selectedWorkspaceId}
                  view={view}
                  workspaces={list}
                />
                <div className="rounded-md border border-border">
                  <WorkspaceDetail
                    onContinueSession={onContinueSession}
                    onDismissReconnectError={() => selected && reconnectMutations.clear(selected.workspaceId)}
                    onNewSession={onNewSession}
                    onOpenSshSettings={onOpenSshSettings}
                    onReconnect={(connectionId) => selected && void reconnect(selected.workspaceId, connectionId)}
                    reconnectMutation={selected ? reconnectMutations.get(selected.workspaceId) : undefined}
                    workspace={selected}
                  />
                </div>
              </div>
            );
          }}
        </AsyncBoundary>
      </div>
    </section>
  );
}
