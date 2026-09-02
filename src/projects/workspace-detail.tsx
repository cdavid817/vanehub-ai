import { Folder, FolderGit2, Server } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { formatAppDateTime } from "../i18n/format";
import { lifecycleDotClass, lifecycleLabelKey, lifecycleTone } from "../lib/session-lifecycle";
import { useWorkspacePlanLinks } from "./use-workspace-plan-links";
import type { WorkspaceAvailability, WorkspaceSummary, WorkspaceTrust } from "./workspace-summary";

const availabilityTone: Record<WorkspaceAvailability, "success" | "warning" | "danger"> = {
  available: "success",
  disconnected: "warning",
  missing: "danger",
};

// Mirrors workspace-card.tsx's own total mapping. Kept as a small local duplicate rather than an
// export from that file -- 13.7's brief asks to leave workspace-card.tsx's own rendering alone
// beyond the selection affordance it already needed.
const trustTone: Record<WorkspaceTrust, "success" | "muted" | "danger"> = {
  revoked: "danger",
  trusted: "success",
  unknown: "muted",
  untrusted: "danger",
};

function WorkspaceKindIcon({ workspace }: { workspace: WorkspaceSummary }) {
  if (workspace.kind === "ssh") return <Server aria-hidden="true" className="h-4 w-4 shrink-0 text-muted-foreground" />;
  if (workspace.git?.repository) return <FolderGit2 aria-hidden="true" className="h-4 w-4 shrink-0 text-muted-foreground" />;
  return <Folder aria-hidden="true" className="h-4 w-4 shrink-0 text-muted-foreground" />;
}

/**
 * `workspace.git` is `undefined` for every SSH row (never derived this increment) and always an
 * object for local rows (see `workspace-aggregation.ts`), so "undefined" and "repository: false"
 * are genuinely different facts -- "never checked" vs. "checked, and it is not one" -- and must not
 * collapse onto the same rendered copy.
 */
function repositoryState(workspace: WorkspaceSummary): "yes" | "no" | "unknown" {
  if (!workspace.git) return "unknown";
  return workspace.git.repository ? "yes" : "no";
}

/**
 * The related-Plan-links section (13.7): a real client-side join over the two existing services
 * Work Board / Goal Center already use, scoped to the one selected workspace -- see
 * `use-workspace-plan-links.ts` for why this is its own component (it needs its own hook call,
 * keyed by `workspaceId`, independent of the rest of the detail panel re-rendering).
 */
function WorkspacePlanSection({ workspaceId }: { workspaceId: string }) {
  const { t } = useTranslation();
  const { data, error, loading } = useWorkspacePlanLinks(workspaceId);
  const isEmpty = data && data.workItems.length === 0 && data.goals.length === 0;

  return (
    <div className="grid gap-2 rounded-md border border-border bg-muted/10 p-3">
      <h3 className="text-sm font-semibold">{t("projects.detail.plan.title")}</h3>
      {loading ? <p className="text-xs text-muted-foreground" role="status">{t("workbenchUi.async.loading")}</p> : null}
      {error ? <p className="text-xs text-destructive" role="alert">{error}</p> : null}
      {isEmpty ? <p className="text-xs text-muted-foreground">{t("projects.detail.plan.empty")}</p> : null}
      {data?.workItems.length ? (
        <div className="grid gap-1">
          <h4 className="text-xs font-medium text-muted-foreground">{t("projects.detail.plan.workItemsLabel")}</h4>
          <ul aria-label={t("projects.detail.plan.workItemsLabel")} className="grid gap-1">
            {data.workItems.map((item) => (
              <li className="flex items-center justify-between gap-2 rounded border border-border px-2 py-1" key={item.id}>
                <span className="min-w-0 flex-1 truncate text-xs">{item.title}</span>
                <Badge tone="muted">{t(`todoBoard.stage.${item.stage}`)}</Badge>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      {data?.goals.length ? (
        <div className="grid gap-1">
          <h4 className="text-xs font-medium text-muted-foreground">{t("projects.detail.plan.goalsLabel")}</h4>
          <ul aria-label={t("projects.detail.plan.goalsLabel")} className="grid gap-1">
            {data.goals.map((goal) => (
              <li className="flex items-center justify-between gap-2 rounded border border-border px-2 py-1" key={goal.id}>
                <span className="min-w-0 flex-1 truncate text-xs">{goal.title}</span>
                <Badge tone="muted">{t(`goals.status.${goal.derivedStatus}`)}</Badge>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}

export interface WorkspaceDetailProps {
  workspace: WorkspaceSummary | null;
}

/**
 * Task 13.7: identity, trust, Git/worktree, recent Session, active Runs, and related Plan/Quality
 * links for the workspace selected in `projects.tsx`'s own master-detail split (mirrors
 * `GoalCenter`/`GoalDetail`'s established layout, `goal-center.tsx`/`goal-detail.tsx`). Read-only:
 * no actions, no mutations -- state-aware actions are task 13.8's own separate, larger scope.
 *
 * Every field renders *something* rather than silently disappearing when the underlying data is
 * absent -- trust for local rows, git detail for SSH rows, active Runs, and Quality links are all
 * confirmed gaps in `WorkspaceSummary`/today's services (see the type's own doc comments and this
 * file's section components below), not bugs, and each one says so explicitly instead of reading
 * like an unfinished feature.
 */
export function WorkspaceDetail({ workspace }: WorkspaceDetailProps) {
  const { i18n, t } = useTranslation();

  if (!workspace) {
    return (
      <div className="grid h-full place-items-center p-4 text-center text-xs text-muted-foreground" data-testid="workspace-detail-empty">
        {t("projects.detail.empty")}
      </div>
    );
  }

  const session = workspace.recentSession;

  return (
    <section aria-labelledby="workspace-detail-title" className="grid content-start gap-3 overflow-y-auto p-3" data-testid="workspace-detail">
      <header className="grid gap-2 rounded-md border border-border bg-muted/10 p-3">
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div className="flex min-w-0 items-center gap-2">
            <WorkspaceKindIcon workspace={workspace} />
            <div className="min-w-0">
              <h2 className="truncate text-base font-semibold" id="workspace-detail-title">{workspace.displayName}</h2>
              <p className="truncate text-xs text-muted-foreground" title={workspace.displayPath}>{workspace.displayPath}</p>
            </div>
          </div>
          <Badge tone={availabilityTone[workspace.availability]}>{t(`projects.availability.${workspace.availability}`)}</Badge>
        </div>
        <p className="text-xs text-muted-foreground">{t(workspace.kind === "ssh" ? "projects.kind.ssh" : "projects.kind.local")}</p>
        {workspace.lastOpenedAt ? (
          <p className="text-[11px] text-muted-foreground">
            {t("projects.lastOpened", { date: formatAppDateTime(workspace.lastOpenedAt, i18n.language, { dateStyle: "medium" }) })}
          </p>
        ) : null}
      </header>

      <div className="grid gap-1 rounded-md border border-border bg-muted/10 p-3">
        <h3 className="text-sm font-semibold">{t("projects.detail.trust.title")}</h3>
        {workspace.trust ? (
          <Badge className="w-fit" tone={trustTone[workspace.trust]}>{t(`projects.trust.${workspace.trust}`)}</Badge>
        ) : (
          <p className="text-xs text-muted-foreground">{t("projects.detail.trust.notApplicable")}</p>
        )}
      </div>

      <div className="grid gap-1 rounded-md border border-border bg-muted/10 p-3">
        <h3 className="text-sm font-semibold">{t("projects.detail.git.title")}</h3>
        <p className="text-xs text-muted-foreground">{t(`projects.detail.git.repository.${repositoryState(workspace)}`)}</p>
        <p className="text-xs text-muted-foreground">{t("projects.detail.git.notTracked")}</p>
      </div>

      <div className="grid gap-1 rounded-md border border-border bg-muted/10 p-3">
        <h3 className="text-sm font-semibold">{t("projects.detail.recentSession.title")}</h3>
        {session ? (
          <div className="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
            <span aria-hidden="true" className={`h-2 w-2 shrink-0 rounded-full ${lifecycleDotClass[lifecycleTone(session.lifecycleState)]}`} />
            <span className="min-w-0 flex-1 truncate">{session.title}</span>
            <span className="shrink-0">{t(lifecycleLabelKey(session.lifecycleState))}</span>
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">{t("projects.recentSession.none")}</p>
        )}
      </div>

      <div className="grid gap-1 rounded-md border border-border bg-muted/10 p-3">
        <h3 className="text-sm font-semibold">{t("projects.detail.activeRuns.title")}</h3>
        <p className="text-xs text-muted-foreground">{t("projects.detail.activeRuns.unavailable")}</p>
      </div>

      <WorkspacePlanSection workspaceId={workspace.workspaceId} />

      <div className="grid gap-1 rounded-md border border-border bg-muted/10 p-3">
        <h3 className="text-sm font-semibold">{t("projects.detail.quality.title")}</h3>
        <p className="text-xs text-muted-foreground">{t("projects.detail.quality.unavailable")}</p>
      </div>
    </section>
  );
}
