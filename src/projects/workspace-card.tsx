import { Folder, FolderGit2, Server } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { formatAppDateTime } from "../i18n/format";
import { lifecycleDotClass, lifecycleLabelKey, lifecycleTone } from "../lib/session-lifecycle";
import { cn } from "../lib/utils";
import type { WorkspaceAvailability, WorkspaceSummary, WorkspaceTrust } from "./workspace-summary";

const availabilityTone: Record<WorkspaceAvailability, "success" | "warning" | "danger"> = {
  available: "success",
  disconnected: "warning",
  missing: "danger",
};

// "untrusted"/"revoked" are included so this mapping stays total even though this increment's own
// derivation logic never produces them (see workspace-summary.ts).
const trustTone: Record<WorkspaceTrust, "success" | "muted" | "danger"> = {
  revoked: "danger",
  trusted: "success",
  unknown: "muted",
  untrusted: "danger",
};

function WorkspaceIcon({ workspace }: { workspace: WorkspaceSummary }) {
  if (workspace.kind === "ssh") return <Server aria-hidden="true" className="h-4 w-4 shrink-0 text-muted-foreground" />;
  if (workspace.git?.repository) return <FolderGit2 aria-hidden="true" className="h-4 w-4 shrink-0 text-muted-foreground" />;
  return <Folder aria-hidden="true" className="h-4 w-4 shrink-0 text-muted-foreground" />;
}

export function WorkspaceCard({ workspace }: { workspace: WorkspaceSummary }) {
  const { i18n, t } = useTranslation();
  const session = workspace.recentSession;

  return (
    <article className="ucd-card grid gap-2 rounded-lg p-3" data-testid={`workspace-${workspace.workspaceId}`}>
      <div className="flex items-start justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <WorkspaceIcon workspace={workspace} />
          <h3 className="min-w-0 truncate text-sm font-semibold">{workspace.displayName}</h3>
        </div>
        <Badge tone={availabilityTone[workspace.availability]}>{t(`projects.availability.${workspace.availability}`)}</Badge>
      </div>

      <p className="truncate text-xs text-muted-foreground" title={workspace.displayPath}>{workspace.displayPath}</p>

      {workspace.trust || workspace.git?.repository ? (
        <div className="flex flex-wrap items-center gap-1.5">
          {workspace.trust ? <Badge tone={trustTone[workspace.trust]}>{t(`projects.trust.${workspace.trust}`)}</Badge> : null}
          {workspace.git?.repository ? <Badge tone="muted">{t("projects.git.repository")}</Badge> : null}
        </div>
      ) : null}

      {session ? (
        <div className="flex min-w-0 items-center gap-1.5 border-t border-border/60 pt-2 text-xs text-muted-foreground">
          <span aria-hidden="true" className={cn("h-2 w-2 shrink-0 rounded-full", lifecycleDotClass[lifecycleTone(session.lifecycleState)])} />
          <span className="min-w-0 flex-1 truncate">{session.title}</span>
          <span className="shrink-0">{t(lifecycleLabelKey(session.lifecycleState))}</span>
        </div>
      ) : (
        <p className="border-t border-border/60 pt-2 text-xs text-muted-foreground">{t("projects.recentSession.none")}</p>
      )}

      {workspace.lastOpenedAt ? (
        <p className="text-[11px] text-muted-foreground">
          {t("projects.lastOpened", { date: formatAppDateTime(workspace.lastOpenedAt, i18n.language, { dateStyle: "medium" }) })}
        </p>
      ) : null}
    </article>
  );
}
