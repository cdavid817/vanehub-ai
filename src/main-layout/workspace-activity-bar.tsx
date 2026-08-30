import { BarChart3, CircleHelp, Columns3, FolderKanban, MessagesSquare, Radar, Settings } from "lucide-react";
import { cn } from "../lib/utils";
import type { WorkbenchDestination } from "./workbench-route";

export interface WorkspaceActivityBarLabels {
  navigation: string;
  sessions: string;
  expandSessions: string;
  collapseSessions: string;
  projects: string;
  runs: string;
  plan: string;
  quality: string;
  settings: string;
  help: string;
}

interface WorkspaceActivityBarProps {
  activeDestination: WorkbenchDestination;
  labels: WorkspaceActivityBarLabels;
  onOpenSettings: () => void;
  onHelp: () => void;
  onSessions: () => void;
  onProjects: () => void;
  onRuns: () => void;
  onPlan: () => void;
  onQuality: () => void;
  sessionSidebarExpanded: boolean;
}

/** Every entry is icon-only, so each one needs a localized accessible name and tooltip. */
export function workspaceActivityBarLabels(t: (key: string) => string): WorkspaceActivityBarLabels {
  return {
    navigation: t("layout.activityBar.label"),
    sessions: t("layout.activityBar.sessions"),
    expandSessions: t("layout.activityBar.expandSessions"),
    collapseSessions: t("layout.activityBar.collapseSessions"),
    projects: t("layout.activityBar.projects"),
    runs: t("layout.activityBar.runs"),
    plan: t("layout.activityBar.plan"),
    quality: t("layout.activityBar.quality"),
    settings: t("layout.activityBar.settings"),
    help: t("layout.activityBar.help"),
  };
}

const activityButtonClass =
  "ucd-interactive flex h-10 w-10 items-center justify-center rounded-md border border-transparent text-muted-foreground outline-hidden focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background";

function destinationClass(active: boolean) {
  return cn(activityButtonClass, active && "border-primary bg-[hsl(var(--nav-active-soft))] text-primary");
}

/**
 * design.md Decision 1: five stable business domains, replacing the previous nine-entry bar
 * (sessions/loops/work-board/goals/evaluations/mission-control/scheduled-tasks/settings/help).
 * Loops, Schedules, Board, Goals, Evaluation, and Mission Control are reachable through each
 * domain's own secondary navigation instead of a dedicated primary entry — see
 * `WorkspaceActivityBar`'s "Open Loops/Scheduled tasks from activity bar" scenarios in
 * `specs/main-layout-ui/spec.md`.
 */
export function WorkspaceActivityBar({
  activeDestination,
  labels,
  onOpenSettings,
  onHelp,
  onSessions,
  onProjects,
  onRuns,
  onPlan,
  onQuality,
  sessionSidebarExpanded,
}: WorkspaceActivityBarProps) {
  const sessionsLabel = sessionSidebarExpanded ? labels.collapseSessions : labels.expandSessions;

  return (
    <nav aria-label={labels.navigation} className="ucd-activity-bar flex w-12 shrink-0 flex-col items-center px-1 py-2">
      <div className="flex flex-col items-center gap-1" data-activity-group="primary">
        <button
          aria-controls="workspace-session-sidebar"
          aria-expanded={sessionSidebarExpanded}
          aria-label={sessionsLabel}
          className={destinationClass(activeDestination === "sessions")}
          onClick={onSessions}
          title={sessionsLabel}
          type="button"
        >
          <MessagesSquare aria-hidden="true" className="h-5 w-5" />
        </button>
        <button aria-controls="workbench-route-outlet" aria-label={labels.projects} className={destinationClass(activeDestination === "projects")} onClick={onProjects} title={labels.projects} type="button">
          <FolderKanban aria-hidden="true" className="h-5 w-5" />
        </button>
        <button aria-controls="workbench-route-outlet" aria-label={labels.runs} className={destinationClass(activeDestination === "runs")} onClick={onRuns} title={labels.runs} type="button">
          <Radar aria-hidden="true" className="h-5 w-5" />
        </button>
        <button aria-controls="workbench-route-outlet" aria-label={labels.plan} className={destinationClass(activeDestination === "plan")} onClick={onPlan} title={labels.plan} type="button">
          <Columns3 aria-hidden="true" className="h-5 w-5" />
        </button>
        <button aria-controls="workbench-route-outlet" aria-label={labels.quality} className={destinationClass(activeDestination === "quality")} onClick={onQuality} title={labels.quality} type="button">
          <BarChart3 aria-hidden="true" className="h-5 w-5" />
        </button>
      </div>
      <div className="mt-auto flex flex-col items-center gap-1" data-activity-group="utility">
        <button aria-label={labels.settings} className={activityButtonClass} data-testid="desktop-smoke-settings" onClick={onOpenSettings} title={labels.settings} type="button">
          <Settings aria-hidden="true" className="h-5 w-5" />
        </button>
        <button aria-label={labels.help} className={activityButtonClass} onClick={onHelp} title={labels.help} type="button">
          <CircleHelp aria-hidden="true" className="h-5 w-5" />
        </button>
      </div>
    </nav>
  );
}
