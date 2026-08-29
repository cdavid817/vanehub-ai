import { Activity, BarChart3, CalendarClock, CircleHelp, Columns3, MessagesSquare, Radar, Repeat2, Settings, Target } from "lucide-react";
import { cn } from "../lib/utils";

export interface WorkspaceActivityBarLabels {
  navigation: string;
  sessions: string;
  expandSessions: string;
  collapseSessions: string;
  loops: string;
  scheduledTasks: string;
  todoBoard: string;
  goals: string;
  evaluations: string;
  missionControl: string;
  systemActivity: string;
  settings: string;
  help: string;
}

interface WorkspaceActivityBarProps {
  activeDestination: "sessions" | "loops" | "work-board" | "goals" | "evaluations" | "mission-control" | "system-activity";
  labels: WorkspaceActivityBarLabels;
  onOpenSettings: () => void;
  onHelp: () => void;
  onLoops: () => void;
  onSessions: () => void;
  onScheduledTasks: () => void;
  onWorkBoard: () => void;
  onGoals: () => void;
  onEvaluations: () => void;
  onMissionControl: () => void;
  onSystemActivity: () => void;
  /** Total unread system-activity items; renders as a badge on the entry. */
  systemActivityUnread: number;
  sessionSidebarExpanded: boolean;
}

/** Every entry is icon-only, so each one needs a localized accessible name and tooltip. */
export function workspaceActivityBarLabels(t: (key: string) => string): WorkspaceActivityBarLabels {
  return {
    navigation: t("layout.activityBar.label"),
    sessions: t("layout.activityBar.sessions"),
    expandSessions: t("layout.activityBar.expandSessions"),
    collapseSessions: t("layout.activityBar.collapseSessions"),
    loops: t("layout.activityBar.loops"),
    scheduledTasks: t("layout.activityBar.scheduledTasks"),
    todoBoard: t("layout.activityBar.todoBoard"),
    goals: t("layout.activityBar.goals"),
    evaluations: t("layout.activityBar.evaluations"),
    missionControl: t("layout.activityBar.missionControl"),
    systemActivity: t("layout.activityBar.systemActivity"),
    settings: t("layout.activityBar.settings"),
    help: t("layout.activityBar.help"),
  };
}

const activityButtonClass =
  "ucd-interactive flex h-10 w-10 items-center justify-center rounded-md border border-transparent text-muted-foreground outline-hidden focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background";

export function WorkspaceActivityBar({
  activeDestination,
  labels,
  onOpenSettings,
  onHelp,
  onLoops,
  onSessions,
  onScheduledTasks,
  onWorkBoard,
  onGoals,
  onEvaluations,
  onMissionControl,
  onSystemActivity,
  systemActivityUnread,
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
          className={cn(activityButtonClass, activeDestination === "sessions" && "border-primary bg-[hsl(var(--nav-active-soft))] text-primary")}
          onClick={onSessions}
          title={sessionsLabel}
          type="button"
        >
          <MessagesSquare aria-hidden="true" className="h-5 w-5" />
        </button>
        <button
          aria-controls="loop-center"
          aria-label={labels.loops}
          className={cn(activityButtonClass, activeDestination === "loops" && "border-primary bg-[hsl(var(--nav-active-soft))] text-primary")}
          onClick={onLoops}
          title={labels.loops}
          type="button"
        >
          <Repeat2 aria-hidden="true" className="h-5 w-5" />
        </button>
        <button aria-controls="work-board" aria-label={labels.todoBoard} className={cn(activityButtonClass, activeDestination === "work-board" && "border-primary bg-[hsl(var(--nav-active-soft))] text-primary")} onClick={onWorkBoard} title={labels.todoBoard} type="button"><Columns3 aria-hidden="true" className="h-5 w-5" /></button>
        <button aria-controls="goal-center" aria-label={labels.goals} className={cn(activityButtonClass, activeDestination === "goals" && "border-primary bg-[hsl(var(--nav-active-soft))] text-primary")} onClick={onGoals} title={labels.goals} type="button"><Target aria-hidden="true" className="h-5 w-5" /></button>
        <button aria-controls="evaluation-center" aria-label={labels.evaluations} className={cn(activityButtonClass, activeDestination === "evaluations" && "border-primary bg-[hsl(var(--nav-active-soft))] text-primary")} onClick={onEvaluations} title={labels.evaluations} type="button"><BarChart3 aria-hidden="true" className="h-5 w-5" /></button>
        <button aria-controls="system-activity" aria-label={labels.systemActivity} className={cn(activityButtonClass, "relative", activeDestination === "system-activity" && "border-primary bg-[hsl(var(--nav-active-soft))] text-primary")} onClick={onSystemActivity} title={labels.systemActivity} type="button">
          <Activity aria-hidden="true" className="h-5 w-5" />
          {systemActivityUnread > 0 ? (
            <span className="absolute -right-0.5 -top-0.5 min-w-4 rounded-full bg-primary px-1 text-center text-[9px] leading-4 text-primary-foreground" data-testid="system-activity-bar-badge">
              {systemActivityUnread > 99 ? "99+" : systemActivityUnread}
            </span>
          ) : null}
        </button>
        <button aria-controls="mission-control" aria-label={labels.missionControl} className={cn(activityButtonClass, activeDestination === "mission-control" && "border-primary bg-[hsl(var(--nav-active-soft))] text-primary")} onClick={onMissionControl} title={labels.missionControl} type="button"><Radar aria-hidden="true" className="h-5 w-5" /></button>
      </div>
      {/* Scheduled tasks opens a dialog rather than switching destination, so it sits apart from
          the four entries that do change what fills the workspace. */}
      <div className="mt-1 flex flex-col items-center gap-1 border-t border-border pt-2" data-activity-group="tools">
        <button aria-haspopup="dialog" aria-label={labels.scheduledTasks} className={activityButtonClass} onClick={onScheduledTasks} title={labels.scheduledTasks} type="button">
          <CalendarClock aria-hidden="true" className="h-5 w-5" />
        </button>
      </div>
      <div className="mt-auto flex flex-col items-center gap-1" data-activity-group="utility">
        <button aria-label={labels.settings} className={activityButtonClass} data-testid="desktop-smoke-settings" onClick={() => onOpenSettings()} title={labels.settings} type="button">
          <Settings aria-hidden="true" className="h-5 w-5" />
        </button>
        <button aria-label={labels.help} className={activityButtonClass} onClick={onHelp} title={labels.help} type="button">
          <CircleHelp aria-hidden="true" className="h-5 w-5" />
        </button>
      </div>
    </nav>
  );
}
