import { LazyFeature, type LazyFeatureLoader } from "../components/lazy-feature";
import type { SettingsPageId } from "../settings/settings-pages";

// Locally declared, not imported from `projects/projects.tsx`: matches how every other
// `*-destination.tsx` in this file types its own lazy-loaded feature's props (see
// `RunsDestination`'s `MissionControlProps`/`LoopCenterProps`) rather than importing a sibling
// feature's own prop type across the destination boundary.
type ProjectsProps = {
  onContinueSession: (sessionId: string) => void;
  onNewSession: (workspace: { workspaceId: string; kind: "local" | "ssh" }) => void;
  onOpenSshSettings: () => void;
};
const loadProjects: LazyFeatureLoader<ProjectsProps> = () => import("../projects/projects")
  .then((module) => ({ default: module.Projects }));

/**
 * Task 13.1: real content replacing the former placeholder (design.md Decision 18).
 *
 * Task 13.8 gave `Projects` its own Continue Session / New Session / Settings actions, each of
 * which needs to reach `MainLayout`'s own `goToSessions`/`onOpenSettings` -- the same shape Runs'
 * own `onMissionControlNavigate`/`onInspectLoop` already forward through `componentProps` here.
 * `onOpenSshSettings` is pre-bound to `"ssh-connections"` here rather than forwarding the general
 * `onOpenSettings(pageId)` signature down into `projects/*`: that keeps `SettingsPageId` knowledge
 * in `main-layout/` and lets `WorkspaceDetail` stay honest about the one real Settings destination
 * a workspace row has today (see workspace-detail.tsx's own doc comment on why local rows have none).
 */
export function ProjectsDestination({
  onContinueSession,
  onNewSessionForWorkspace,
  onOpenSettings,
}: {
  onContinueSession: (sessionId: string) => void;
  onNewSessionForWorkspace: (workspace: { workspaceId: string; kind: "local" | "ssh" }) => void;
  onOpenSettings: (pageId?: SettingsPageId) => void;
}) {
  return (
    <div className="h-full min-h-0 p-2">
      <LazyFeature
        className="h-full min-h-0"
        componentProps={{
          onContinueSession,
          onNewSession: onNewSessionForWorkspace,
          onOpenSshSettings: () => onOpenSettings("ssh-connections"),
        }}
        loader={loadProjects}
      />
    </div>
  );
}
