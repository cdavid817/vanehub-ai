import { useCallback, useEffect, useRef, useState } from "react";
import { sessionShellService } from "../services/runtime-session-shell-client";
import type { SessionShellDescriptor } from "../types/session-workspace-shell-frames";
import { isCloseSettled } from "../types/session-workspace-shell-frames";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

export interface SessionShellsState {
  shells: SessionShellDescriptor[];
  activeShellId: string | null;
  error: WorkspaceErrorKey | null;
  selectShell(shellId: string): void;
  addShell(): Promise<void>;
  renameShell(shellId: string, title: string): Promise<void>;
  closeShell(shellId: string): Promise<void>;
  applyDescriptor(descriptor: SessionShellDescriptor): void;
  clearError(): void;
}

/**
 * The Shells a session is holding, and the four things a user can do to that list.
 *
 * Loaded rather than owned: the registry outlives every view, so this hook reads what is already
 * there and only creates when the session has nothing open. A hook that created a Shell on mount
 * would spawn a process for a tab the user never looked at, and a second one each time they came
 * back.
 */
export function useSessionShells(
  sessionId: string | null,
  seatId: string | null,
  isVisible: boolean,
): SessionShellsState {
  const [shells, setShells] = useState<SessionShellDescriptor[]>([]);
  const [activeShellId, setActiveShellId] = useState<string | null>(null);
  const [error, setError] = useState<WorkspaceErrorKey | null>(null);
  // Whether the registry has answered. An empty list before it has is "not asked yet", and acting
  // on it would open a Shell alongside the ones the session already has.
  const [loaded, setLoaded] = useState(false);
  // Guards the one create this hook is allowed to make per session, so a re-render while the
  // create is in flight does not start a second one.
  const openingRef = useRef<string | null>(null);

  const replaceShell = useCallback((descriptor: SessionShellDescriptor) => {
    setShells((current) => {
      const index = current.findIndex((shell) => shell.shellId === descriptor.shellId);
      if (index === -1) return [...current, descriptor];
      // Older revisions arrive out of order when a state notice races a command response; keeping
      // the newer one is what stops a finished Shell from flickering back to running.
      if (current[index].revision > descriptor.revision) return current;
      const next = [...current];
      next[index] = descriptor;
      return next;
    });
  }, []);

  useEffect(() => {
    setLoaded(false);
    if (!sessionId) {
      setShells([]);
      setActiveShellId(null);
      return;
    }
    let disposed = false;
    setError(null);
    void (async () => {
      try {
        const listed = await sessionShellService.listSessionShells(sessionId);
        if (disposed) return;
        const scoped = listed.filter((shell) => (seatId ? shell.seatId === seatId : true));
        setShells(scoped);
        setActiveShellId((current) =>
          current && scoped.some((shell) => shell.shellId === current)
            ? current
            : (scoped[0]?.shellId ?? null),
        );
      } catch (reason) {
        if (!disposed) setError(workspaceErrorKey(reason));
      } finally {
        // Marked loaded even on failure. A registry that could not be read is still a registry that
        // may be holding Shells, and opening another on top of them would be worse than showing the
        // error and waiting.
        if (!disposed) setLoaded(true);
      }
    })();
    return () => {
      disposed = true;
    };
  }, [seatId, sessionId]);

  // The default Shell opens the first time the tab is actually shown. A hidden tab that spawned a
  // process would charge the user for a panel they never opened.
  useEffect(() => {
    if (!isVisible || !loaded || !sessionId || shells.length > 0) return;
    if (openingRef.current) return;
    openingRef.current = sessionId;
    void (async () => {
      try {
        const shell = await sessionShellService.createSessionShell({
          sessionId,
          rows: 24,
          cols: 80,
          seatId: seatId ?? undefined,
        });
        replaceShell(shell);
        setActiveShellId((current) => current ?? shell.shellId);
      } catch (reason) {
        setError(workspaceErrorKey(reason));
      } finally {
        openingRef.current = null;
      }
    })();
  }, [isVisible, loaded, replaceShell, seatId, sessionId, shells.length]);

  const addShell = useCallback(async () => {
    if (!sessionId || openingRef.current) return;
    // One key per press, held across the call so a retry of this press returns the Shell it already
    // made rather than a second one. Two deliberate presses are two keys and two Shells, which is
    // what the user asked for; the in-flight guard is what stops a double click from being two.
    openingRef.current = crypto.randomUUID();
    try {
      const shell = await sessionShellService.createSessionShell({
        sessionId,
        rows: 24,
        cols: 80,
        seatId: seatId ?? undefined,
        requestId: openingRef.current,
      });
      replaceShell(shell);
      setActiveShellId(shell.shellId);
    } catch (reason) {
      setError(workspaceErrorKey(reason));
    } finally {
      openingRef.current = null;
    }
  }, [replaceShell, seatId, sessionId]);

  const renameShell = useCallback(
    async (shellId: string, title: string) => {
      try {
        replaceShell(await sessionShellService.renameSessionShell({ shellId, title }));
      } catch (reason) {
        setError(workspaceErrorKey(reason));
      }
    },
    [replaceShell],
  );

  const closeShell = useCallback(async (shellId: string) => {
    try {
      const outcome = await sessionShellService.closeSessionShell(shellId);
      // Removing the tab because the promise resolved is the defect this change exists to remove.
      // `reaping` and `close_failed` both resolve, and both mean the process may still be running;
      // dropping the Shell here would take away the only handle left that can retry it.
      if (!isCloseSettled(outcome.disposition)) {
        setShells((current) =>
          current.map((shell) =>
            shell.shellId === shellId
              ? {
                  ...shell,
                  state: outcome.disposition === "reaping" ? "reaping" : "close_failed",
                  reason: outcome.reason ?? shell.reason,
                  revision: shell.revision + 1,
                }
              : shell,
          ),
        );
        return;
      }
      setShells((current) => current.filter((shell) => shell.shellId !== shellId));
      setActiveShellId((current) => (current === shellId ? null : current));
    } catch (reason) {
      setError(workspaceErrorKey(reason));
    }
  }, []);

  // Falls to whatever is left rather than to nothing, so closing the active Shell does not empty
  // the panel while other Shells are still open.
  useEffect(() => {
    if (activeShellId || shells.length === 0) return;
    setActiveShellId(shells[0].shellId);
  }, [activeShellId, shells]);

  return {
    shells,
    activeShellId,
    error,
    selectShell: setActiveShellId,
    addShell,
    renameShell,
    closeShell,
    applyDescriptor: replaceShell,
    clearError: () => setError(null),
  };
}
