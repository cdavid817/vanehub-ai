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

export interface WorkspaceCardProps {
  workspace: WorkspaceSummary;
  /** Whether this card is the one currently shown in `WorkspaceDetail` (13.7's master-detail split). */
  selected: boolean;
  onSelect: () => void;
}

export function WorkspaceCard({ onSelect, selected, workspace }: WorkspaceCardProps) {
  const { i18n, t } = useTranslation();
  const session = workspace.recentSession;

  return (
    <button
      aria-current={selected}
      className={cn(
        "ucd-card grid w-full gap-2 rounded-lg p-3 text-left transition-opacity hover:opacity-90",
        // `.ucd-card` is unlayered CSS (styles.css), which always wins over a layered Tailwind
        // utility of the same property regardless of source order -- the `!` suffix (Tailwind v4
        // important-modifier syntax, already used this way in notification-center.tsx) is the
        // established way around that in this codebase, not a workaround invented here.
        selected && "border-primary!",
      )}
      data-testid={`workspace-${workspace.workspaceId}`}
      onClick={onSelect}
      type="button"
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <WorkspaceIcon workspace={workspace} />
          {/* A plain span, not a heading: heading elements are not valid phrasing content inside
              a <button>, and this row is now one (matches GoalCenter's own row-title span). */}
          <span className="min-w-0 truncate text-sm font-semibold">{workspace.displayName}</span>
        </div>
        <Badge tone={availabilityTone[workspace.availability]}>{t(`projects.availability.${workspace.availability}`)}</Badge>
      </div>

      {/* 20.16: `displayPath` is filesystem-sourced (local disk or a resolved SSH `user@host:path`),
          not app-authored -- `<bdi>` keeps a strong-RTL or mixed-script path segment from reading
          this row's own fixed-direction punctuation out of order. Standard HTML isolation element,
          no new CSS. */}
      <p className="truncate text-xs text-muted-foreground" title={workspace.displayPath}><bdi>{workspace.displayPath}</bdi></p>

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
    </button>
  );
}
