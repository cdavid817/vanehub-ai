import { RefreshCw } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { AsyncBoundary } from "../ui/async/AsyncBoundary";
import type { AsyncViewState } from "../ui/async/async-view-state";
import { EmptyState } from "../ui/empty-state/EmptyState";
import { useProjectWorkspaces } from "./use-project-workspaces";
import { selectWorkspaceView, workspaceViews, type WorkspaceView } from "./workspace-filter";
import { WorkspaceCard } from "./workspace-card";
import type { WorkspaceSummary } from "./workspace-summary";

/**
 * Renders `workspaces` for the active view. Split out from `Projects` so the "only 'unavailable'
 * can be filtered-empty while the raw list is not" reasoning (see workspace-filter.ts) has one
 * place to live instead of being re-derived at each call site.
 */
function WorkspaceViewList({ view, workspaces }: { view: WorkspaceView; workspaces: WorkspaceSummary[] }) {
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
    <ul aria-label={t("projects.listLabel")} className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
      {visible.map((workspace) => <li key={workspace.workspaceId}><WorkspaceCard workspace={workspace} /></li>)}
    </ul>
  );
}

/**
 * Read-only aggregation entry point for §13 (design.md Decision 18). Real content replacing the
 * former placeholder: local project history, SSH known workspaces, SSH connection trust, and
 * recent sessions, joined client-side by `useProjectWorkspaces` — no new Tauri command or
 * writable cross-domain table. Scoped to 13.1/13.5/13.6/13.11 plus a partial 13.4 (recent/all/
 * unavailable only); detail panel, state-aware actions, and the remaining views are follow-up
 * work — see this increment's own report for the full list of deferred tasks.
 */
export function Projects() {
  const { t } = useTranslation();
  const { data, error, loading, reload } = useProjectWorkspaces();
  const [view, setView] = useState<WorkspaceView>("recent");

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
          {(list) => <WorkspaceViewList view={view} workspaces={list} />}
        </AsyncBoundary>
      </div>
    </section>
  );
}
