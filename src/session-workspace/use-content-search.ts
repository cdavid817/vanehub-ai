import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import type {
  WorkspaceContentMatch,
  WorkspaceSearchCoverage,
} from "../types/session-workspace-inspection";

/**
 * How long a keystroke waits before it becomes a search.
 *
 * Longer than Quick Open's, because the work is not comparable: a path search reads directory
 * entries, this one reads file contents. A reader mid-word would otherwise start a full workspace
 * scan for every letter.
 */
const KEYSTROKE_DELAY_MS = 300;

/**
 * Distinguishes one panel's searches from another's.
 *
 * Module scope rather than per hook, because two panels — two sessions side by side, or one
 * remounting — must not collide. The registry on the native side is keyed by search id alone, so a
 * second panel reusing the first's id would supersede a search nobody replaced, and a cancel meant
 * for one would stop the other.
 */
let nextPanelId = 0;

interface ContentSearchState {
  matches: WorkspaceContentMatch[];
  coverage: WorkspaceSearchCoverage | null;
  isSearching: boolean;
  failed: boolean;
}

const EMPTY: ContentSearchState = {
  matches: [],
  coverage: null,
  isSearching: false,
  failed: false,
};

/**
 * Content search, with a cancellation that actually reaches the search.
 *
 * Unlike Quick Open, dropping a superseded answer is not enough here. A content search reads every
 * file in a workspace, and a reader who keeps typing would otherwise leave a trail of full scans
 * running on a machine that has already been told nobody wants them.
 *
 * One id for the panel's whole life, reused for every keystroke, rather than a fresh id per attempt.
 * That is what makes supersession the native side's job: registering under an id already in flight
 * cancels the previous generation under the same lock, so there is no window where two scans are
 * running and neither has been told to stop. A per-attempt id would leave every scan looking
 * independent, and the only thing stopping the old one would be a cancel racing the new request.
 */
export function useContentSearch(sessionId: string | null, isOpen: boolean) {
  const [query, setQuery] = useState("");
  const [state, setState] = useState<ContentSearchState>(EMPTY);
  const attempt = useRef(0);
  const searchId = useRef<string | null>(null);
  if (searchId.current === null) searchId.current = `content-${(nextPanelId += 1)}`;
  const panelSearchId = searchId.current;
  /**
   * The newest generation whose answer has been applied.
   *
   * Two requests can be in flight and arrival order is not issue order, so "the last response wins"
   * is the rule that puts a stale answer over a fresh one. Zero because the native counter starts at
   * one, so the first real answer always beats it.
   */
  const applied = useRef(0);
  const running = useRef(false);

  const cancelInFlight = useCallback(() => {
    if (!running.current) return;
    running.current = false;
    // Failures are swallowed: the search either stopped or had already finished, and neither is
    // something to put on screen. The reader has moved on in both cases.
    void agentService.cancelWorkspaceSearch(panelSearchId).catch(() => {});
  }, [panelSearchId]);

  useEffect(() => {
    if (!isOpen) {
      setQuery("");
      cancelInFlight();
    }
  }, [cancelInFlight, isOpen]);

  useEffect(() => {
    if (!isOpen || !sessionId || !query.trim()) {
      cancelInFlight();
      applied.current = 0;
      setState(EMPTY);
      return;
    }
    const request = (attempt.current += 1);
    setState((current) => ({ ...current, isSearching: true, failed: false }));

    const timer = setTimeout(() => {
      running.current = true;
      agentService
        .searchWorkspaceContent({ sessionId, query, searchId: panelSearchId })
        .then((result) => {
          if (attempt.current !== request) return;
          // Checked even though the attempt matched. The attempt counter only knows what this hook
          // asked for; the generation is what the native registry actually ran, and a request whose
          // predecessor is still winding down can have its answer arrive second.
          if (result.generation < applied.current) return;
          applied.current = result.generation;
          running.current = false;
          setState({
            matches: result.matches,
            coverage: result.coverage,
            isSearching: false,
            failed: false,
          });
        })
        .catch(() => {
          if (attempt.current !== request) return;
          running.current = false;
          setState({ ...EMPTY, failed: true });
        });
    }, KEYSTROKE_DELAY_MS);

    return () => {
      clearTimeout(timer);
    };
  }, [cancelInFlight, isOpen, panelSearchId, query, sessionId]);

  // A search still running when the panel unmounts has nobody waiting on it. Without this it would
  // keep reading files until it hit its own bound, on a machine that has already moved on.
  useEffect(() => cancelInFlight, [cancelInFlight]);

  return useMemo(
    () => ({ ...state, cancel: cancelInFlight, query, setQuery }),
    [cancelInFlight, query, state],
  );
}
