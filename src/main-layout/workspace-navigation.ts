import type { SessionTabId } from "../session-workspace/session-tab-bar";
import type { MissionControlNavigationTarget } from "../types/mission-control";
import type { WorkspaceLocation } from "./workspace-route";

export type WorkspaceNavigationTarget =
  | { kind: "workspace"; location: Partial<WorkspaceLocation>; sessionTab?: SessionTabId }
  | { kind: "settings"; pageId: "evaluation" };

/**
 * Where a Mission Control navigation target lands now that Loops, Goals, and Evaluations are no
 * longer destinations of their own. The contract emitted by the hosted surface is unchanged;
 * only the mapping to a location lives here.
 */
export function resolveMissionControlNavigation(target: MissionControlNavigationTarget): WorkspaceNavigationTarget {
  switch (target.kind) {
    case "loop":
      return { kind: "workspace", location: { destination: "automations", view: "loops" } };
    case "goal":
      return { kind: "workspace", location: { destination: "automations", view: "goals" } };
    case "evaluation":
      return { kind: "settings", pageId: "evaluation" };
    case "review":
      return {
        kind: "workspace",
        location: { destination: "sessions", sessionId: target.sessionId ?? target.id, creatingSession: false },
        sessionTab: "changes",
      };
    default:
      return { kind: "workspace", location: { destination: "sessions", sessionId: target.sessionId ?? target.id, creatingSession: false } };
  }
}
