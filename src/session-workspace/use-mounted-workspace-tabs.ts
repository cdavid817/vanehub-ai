import { useCallback, useState } from "react";
import type { SessionPrimarySurfaceId } from "./session-surface-registry";

const WORK_ONLY: readonly SessionPrimarySurfaceId[] = Object.freeze(["work"]);

interface MountedTabs {
  sessionId: string | null;
  tabs: readonly SessionPrimarySurfaceId[];
}

/**
 * Which primary surfaces are mounted, and when that set goes back to just the conversation.
 *
 * Runtime surfaces (Terminal History/Shell/Logs/Traces) are not tracked here — the shared
 * `RuntimePanel` shell (`src/ui/runtime-panel`) already lazy-mounts-once and keeps its own tabs
 * mounted internally; this hook only ever needed to cover the four primary surfaces this session
 * workspace switches between directly.
 *
 * The active surface is always in the set. It has to be derived rather than only added by the
 * click handler, because a surface can become active without anyone clicking it: a cross-panel
 * jump moves the active surface through the evidence scope, and a panel that was never mounted
 * showed a selected tab above an empty body.
 *
 * The reset happens during render rather than in an effect. An effect runs after the children have
 * rendered, which leaves the previous session's panels mounted for one frame — and a mounted Files
 * panel is a request, issued against a session the user has already left and answered into a panel
 * that is about to be torn down.
 */
export function useMountedWorkspaceTabs(
  sessionId: string | null,
  activeSurface: SessionPrimarySurfaceId,
): {
  mountedTabs: readonly SessionPrimarySurfaceId[];
  mount: (id: SessionPrimarySurfaceId) => void;
} {
  const [stored, setStored] = useState<MountedTabs>(() => ({ sessionId, tabs: WORK_ONLY }));
  if (stored.sessionId !== sessionId) setStored({ sessionId, tabs: WORK_ONLY });
  const settled = stored.sessionId === sessionId ? stored.tabs : WORK_ONLY;
  const mountedTabs = settled.includes(activeSurface) ? settled : [...settled, activeSurface];

  const mount = useCallback(
    (id: SessionPrimarySurfaceId) => {
      setStored((current) => {
        if (current.sessionId !== sessionId) {
          return { sessionId, tabs: id === "work" ? WORK_ONLY : ["work", id] };
        }
        if (current.tabs.includes(id)) return current;
        return { sessionId, tabs: [...current.tabs, id] };
      });
    },
    [sessionId],
  );

  return { mountedTabs, mount };
}
