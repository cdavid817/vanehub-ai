import { useCallback, useState } from "react";
import type { SessionTabId } from "./session-tab-bar";

const CHAT_ONLY: readonly SessionTabId[] = Object.freeze(["chat"]);

interface MountedTabs {
  sessionId: string | null;
  tabs: readonly SessionTabId[];
}

/**
 * Which workspace panels are mounted, and when that set goes back to just the conversation.
 *
 * The reset happens during render rather than in an effect. An effect runs after the children have
 * rendered, which leaves the previous session's panels mounted for one frame — and a mounted Logs
 * or Traces panel is a request, issued against a session the user has already left and answered
 * into a panel that is about to be torn down.
 */
export function useMountedWorkspaceTabs(sessionId: string | null): {
  mountedTabs: readonly SessionTabId[];
  mount: (tab: SessionTabId) => void;
} {
  const [stored, setStored] = useState<MountedTabs>(() => ({ sessionId, tabs: CHAT_ONLY }));
  if (stored.sessionId !== sessionId) setStored({ sessionId, tabs: CHAT_ONLY });
  const mountedTabs = stored.sessionId === sessionId ? stored.tabs : CHAT_ONLY;

  const mount = useCallback(
    (tab: SessionTabId) => {
      setStored((current) => {
        if (current.sessionId !== sessionId) {
          return { sessionId, tabs: tab === "chat" ? CHAT_ONLY : ["chat", tab] };
        }
        if (current.tabs.includes(tab)) return current;
        return { sessionId, tabs: [...current.tabs, tab] };
      });
    },
    [sessionId],
  );

  return { mountedTabs, mount };
}
