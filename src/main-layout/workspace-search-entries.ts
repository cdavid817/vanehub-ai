import type { TFunction } from "i18next";
import type { WorkspaceLocation } from "./workspace-route";

export type WorkspaceSurfaceTarget =
  | { kind: "workspace"; location: Pick<WorkspaceLocation, "destination" | "view"> }
  | { kind: "settings"; pageId: "evaluation" };

export interface WorkspaceSurfaceEntry {
  id: string;
  label: string;
  /** Locale-independent words so an English query still finds a surface in a zh-CN UI. */
  keywords: string[];
  target: WorkspaceSurfaceTarget;
}

/**
 * The surfaces that no longer have an activity-bar entry of their own. Listing them here is what
 * keeps them one search away from the top bar rather than one click away that no longer exists.
 */
export function workspaceSurfaceEntries(t: TFunction): WorkspaceSurfaceEntry[] {
  return [
    { id: "inbox", label: t("layout.activityBar.inbox"), keywords: ["inbox", "attention", "mission control"], target: { kind: "workspace", location: { destination: "inbox", view: "attention" } } },
    { id: "board", label: t("todoBoard.title"), keywords: ["board", "todo", "kanban"], target: { kind: "workspace", location: { destination: "inbox", view: "board" } } },
    { id: "activity", label: t("inbox.activityLog"), keywords: ["activity", "log", "system activity"], target: { kind: "workspace", location: { destination: "inbox", view: "attention" } } },
    { id: "loops", label: t("automations.tab.loops"), keywords: ["loops", "loop"], target: { kind: "workspace", location: { destination: "automations", view: "loops" } } },
    { id: "scheduled", label: t("automations.tab.scheduled"), keywords: ["scheduled", "schedule", "cron"], target: { kind: "workspace", location: { destination: "automations", view: "scheduled" } } },
    { id: "goals", label: t("automations.tab.goals"), keywords: ["goals", "goal"], target: { kind: "workspace", location: { destination: "automations", view: "goals" } } },
    { id: "evaluation", label: t("settings.pages.evaluation"), keywords: ["evaluation", "evaluations", "arena", "benchmark"], target: { kind: "settings", pageId: "evaluation" } },
  ];
}

export function matchWorkspaceSurfaces(query: string, entries: WorkspaceSurfaceEntry[]): WorkspaceSurfaceEntry[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [];
  return entries.filter((entry) =>
    entry.label.toLowerCase().includes(needle) || entry.keywords.some((keyword) => keyword.includes(needle) || needle.includes(keyword)));
}
