import { useEffect, useState } from "react";
import { agentService } from "../services/runtime-agent-client";

/**
 * Total unread system-activity items for the workspace navigation badge. A summary, not a live
 * stream: it refreshes when the workspace mounts and whenever the user changes destination —
 * reading inside System Activity is what changes the count.
 */
export function useSystemActivityUnread(destination: string): number {
  const [unread, setUnread] = useState(0);
  useEffect(() => {
    let cancelled = false;
    void agentService
      .listSystemActivitySessions()
      .then((sessions) => {
        if (cancelled) return;
        setUnread(sessions.reduce((total, session) => total + session.unreadCount, 0));
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [destination]);
  return unread;
}
