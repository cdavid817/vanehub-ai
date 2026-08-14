import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useNotifications } from "../notifications/notification-provider";
import type { Session } from "../types/agent";
import type { WorkspaceLocation } from "./workspace-route";

interface WorkspaceSessionRouteOptions {
  activeSessionId: string | null;
  archivedSessions: Session[];
  location: WorkspaceLocation;
  onNavigate: (next: WorkspaceLocation, options?: { replace?: boolean }) => void;
  sessions: Session[];
  switchSession: (session: Session) => void;
}

/**
 * Reconciles the session named by the URL with the active session the backend owns.
 *
 * The dependency list is deliberately all primitives and stable callbacks. `sessions` and
 * `archivedSessions` come from `query.data ?? []`, which allocates a fresh array on every render
 * while the query is loading; depending on them directly made this effect run each render, and
 * because it navigates, that spun into a render loop.
 */
export function useWorkspaceSessionRoute({
  activeSessionId,
  archivedSessions,
  location,
  onNavigate,
  sessions,
  switchSession,
}: WorkspaceSessionRouteOptions) {
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const listsRef = useRef({ archivedSessions, sessions });
  listsRef.current = { archivedSessions, sessions };
  // Bounds a failed switch to one attempt per route change: `useSessionSwitch` restores the
  // previous active session when the backend rejects it, which would otherwise retry forever.
  const attemptedRef = useRef<string | null>(null);

  const { creatingSession, destination, sessionId } = location;
  const listsLoaded = sessions.length > 0 || archivedSessions.length > 0;
  const routeSessionKnown = sessionId !== null
    && (sessions.some((session) => session.id === sessionId)
      || archivedSessions.some((session) => session.id === sessionId));

  useEffect(() => {
    if (destination !== "sessions" || creatingSession) return;
    const base: WorkspaceLocation = { creatingSession: false, destination: "sessions", sessionId: null };

    if (!sessionId) {
      attemptedRef.current = null;
      if (activeSessionId) onNavigate({ ...base, sessionId: activeSessionId }, { replace: true });
      return;
    }
    if (sessionId === activeSessionId) {
      attemptedRef.current = sessionId;
      return;
    }
    if (routeSessionKnown) {
      if (attemptedRef.current === sessionId) return;
      attemptedRef.current = sessionId;
      const { archivedSessions: archived, sessions: active } = listsRef.current;
      const target = active.find((session) => session.id === sessionId)
        ?? archived.find((session) => session.id === sessionId);
      if (target) switchSession(target);
      return;
    }
    // Only give up once a list has actually arrived, or a deep link bounces on first paint.
    if (!listsLoaded) return;
    notify({
      type: "warning",
      title: t("layout.sessionUnavailableTitle"),
      message: t("layout.sessionUnavailableMessage"),
      scope: { kind: "global" },
    });
    onNavigate(base, { replace: true });
  }, [
    activeSessionId, creatingSession, destination, listsLoaded, notify, onNavigate,
    routeSessionKnown, sessionId, switchSession, t,
  ]);
}
