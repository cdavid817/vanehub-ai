import { useEffect, useState } from "react";
import { countInboxBadge } from "./inbox-feed";
import { loadInboxFeed } from "./load-inbox-feed";

/**
 * The Inbox badge for the workspace navigation. A summary, not a live stream: it refreshes when
 * the workspace mounts and whenever the destination changes, because reading inside Inbox and
 * acting on a run are what change the count. Disabled while Inbox itself is on screen, where the
 * surface reports the count it already computed.
 */
export function useInboxUnread(destination: string, { enabled = true }: { enabled?: boolean } = {}): number {
  const [unread, setUnread] = useState(0);
  useEffect(() => {
    if (!enabled) return undefined;
    let cancelled = false;
    void loadInboxFeed()
      .then(({ feed }) => {
        if (!cancelled) setUnread(countInboxBadge(feed));
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [destination, enabled]);
  return unread;
}
