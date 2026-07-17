import { CalendarClock, CircleHelp, MessagesSquare, Settings } from "lucide-react";
import { cn } from "../lib/utils";

export interface WorkspaceActivityBarLabels {
  navigation: string;
  sessions: string;
  expandSessions: string;
  collapseSessions: string;
  scheduledTasks: string;
  settings: string;
  help: string;
}

interface WorkspaceActivityBarProps {
  labels: WorkspaceActivityBarLabels;
  onOpenSettings: () => void;
  onScheduledTasks: () => void;
  onToggleSessions: () => void;
  sessionSidebarExpanded: boolean;
}

const activityButtonClass =
  "ucd-interactive flex h-10 w-10 items-center justify-center rounded-lg border border-transparent text-muted-foreground outline-none focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background";

export function WorkspaceActivityBar({
  labels,
  onOpenSettings,
  onScheduledTasks,
  onToggleSessions,
  sessionSidebarExpanded,
}: WorkspaceActivityBarProps) {
  const sessionsLabel = sessionSidebarExpanded ? labels.collapseSessions : labels.expandSessions;

  return (
    <nav aria-label={labels.navigation} className="ucd-activity-bar flex w-12 shrink-0 flex-col items-center justify-between px-1 py-2">
      <div className="flex flex-col items-center gap-1" data-activity-group="primary">
        <button
          aria-controls="workspace-session-sidebar"
          aria-expanded={sessionSidebarExpanded}
          aria-label={sessionsLabel}
          className={cn(activityButtonClass, sessionSidebarExpanded && "border-primary bg-[hsl(var(--nav-active-soft))] text-primary")}
          onClick={onToggleSessions}
          title={sessionsLabel}
          type="button"
        >
          <MessagesSquare aria-hidden="true" className="h-5 w-5" />
        </button>
        <button aria-label={labels.scheduledTasks} className={activityButtonClass} onClick={onScheduledTasks} title={labels.scheduledTasks} type="button">
          <CalendarClock aria-hidden="true" className="h-5 w-5" />
        </button>
      </div>
      <div className="flex flex-col items-center gap-1" data-activity-group="utility">
        <button aria-label={labels.settings} className={activityButtonClass} onClick={onOpenSettings} title={labels.settings} type="button">
          <Settings aria-hidden="true" className="h-5 w-5" />
        </button>
        <button aria-label={labels.help} className={activityButtonClass} title={labels.help} type="button">
          <CircleHelp aria-hidden="true" className="h-5 w-5" />
        </button>
      </div>
    </nav>
  );
}
