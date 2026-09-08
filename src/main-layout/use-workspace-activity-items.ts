import { Inbox, MessagesSquare, Settings, Workflow } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ActivityItem } from "./workspace-activity-bar";
import { useWorkspaceShortcuts } from "./workspace-shortcuts";
import type { AutomationsView, WorkspaceDestination } from "./workspace-route";

const automationsTabStorageKey = "vanehub.automations.tab.v1";

export function readLastAutomationsView(): AutomationsView {
  if (typeof localStorage === "undefined") return "loops";
  const stored = localStorage.getItem(automationsTabStorageKey);
  return stored === "scheduled" || stored === "goals" ? stored : "loops";
}

export function rememberAutomationsView(view: AutomationsView): void {
  if (typeof localStorage !== "undefined") localStorage.setItem(automationsTabStorageKey, view);
}

interface WorkspaceActivityOptions {
  activeDestination: WorkspaceDestination;
  inboxUnread: number;
  sessionSidebarExpanded: boolean;
  onSessions: () => void;
  onInbox: () => void;
  onAutomations: () => void;
  onSettings: () => void;
}

/**
 * The four entries as configuration, plus their `Mod+1..4` chords. Kept beside the shell rather
 * than inside the bar so the bar stays ignorant of what each entry opens.
 */
export function useWorkspaceActivityItems(options: WorkspaceActivityOptions): { items: ActivityItem[]; utilityItems: ActivityItem[] } {
  const { t } = useTranslation();
  const { activeDestination, inboxUnread, onAutomations, onInbox, onSessions, onSettings, sessionSidebarExpanded } = options;
  const items = useMemo<ActivityItem[]>(() => [
    {
      id: "sessions",
      icon: MessagesSquare,
      label: sessionSidebarExpanded ? t("layout.activityBar.collapseSessions") : t("layout.activityBar.expandSessions"),
      shortcut: "Mod+1",
      ariaControls: "workspace-session-sidebar",
      expanded: sessionSidebarExpanded,
      active: activeDestination === "sessions",
      onSelect: onSessions,
    },
    {
      id: "inbox",
      icon: Inbox,
      label: t("layout.activityBar.inbox"),
      shortcut: "Mod+2",
      ariaControls: "inbox",
      badge: inboxUnread,
      active: activeDestination === "inbox",
      onSelect: onInbox,
    },
    {
      id: "automations",
      icon: Workflow,
      label: t("layout.activityBar.automations"),
      shortcut: "Mod+3",
      ariaControls: "automations",
      active: activeDestination === "automations",
      onSelect: onAutomations,
    },
  ], [activeDestination, inboxUnread, onAutomations, onInbox, onSessions, sessionSidebarExpanded, t]);
  const utilityItems = useMemo<ActivityItem[]>(() => [
    {
      id: "settings",
      icon: Settings,
      label: t("layout.activityBar.settings"),
      shortcut: "Mod+4",
      testId: "desktop-smoke-settings",
      onSelect: onSettings,
    },
  ], [onSettings, t]);
  const bindings = useMemo(() => [...items, ...utilityItems].map(({ onSelect, shortcut }) => ({ onSelect, shortcut })), [items, utilityItems]);
  useWorkspaceShortcuts(bindings);
  return { items, utilityItems };
}
