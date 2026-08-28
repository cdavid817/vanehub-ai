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
 * running on a machine that has already been told nobody wants them. So each search carries an id
 * and the previous one is cancelled by name before the next begins.
 *
 * The id is generated here rather than taken from the caller. It has to be unique per attempt —
 * reusing it would cancel the search being started — and the only thing that knows how many
 * attempts there have been is the thing counting them.
 */
export function useContentSearch(sessionId: string | null, isOpen: boolean) {
  const [query, setQuery] = useState("");
  const [state, setState] = useState<ContentSearchState>(EMPTY);
  const attempt = useRef(0);
  const inFlight = useRef<string | null>(null);

  const cancelInFlight = useCallback(() => {
    const running = inFlight.current;
    if (!running) return;
    inFlight.current = null;
    // Failures are swallowed: the search either stopped or had already finished, and neither is
    // something to put on screen. The reader has moved on in both cases.
    void agentService.cancelWorkspaceSearch(running).catch(() => {});
  }, []);

  useEffect(() => {
    if (!isOpen) {
      setQuery("");
      cancelInFlight();
    }
  }, [cancelInFlight, isOpen]);

  useEffect(() => {
    if (!isOpen || !sessionId || !query.trim()) {
      cancelInFlight();
      setState(EMPTY);
      return;
    }
    const request = (attempt.current += 1);
    const searchId = `content-${request}`;
    setState((current) => ({ ...current, isSearching: true, failed: false }));

    const timer = setTimeout(() => {
      cancelInFlight();
      inFlight.current = searchId;
      agentService
        .searchWorkspaceContent({ sessionId, query, searchId })
        .then((result) => {
          if (attempt.current !== request) return;
          inFlight.current = null;
          setState({
            matches: result.matches,
            coverage: result.coverage,
            isSearching: false,
            failed: false,
          });
        })
        .catch(() => {
          if (attempt.current !== request) return;
          inFlight.current = null;
          setState({ ...EMPTY, failed: true });
        });
    }, KEYSTROKE_DELAY_MS);

    return () => {
      clearTimeout(timer);
    };
  }, [cancelInFlight, isOpen, query, sessionId]);

  // A search still running when the panel unmounts has nobody waiting on it. Without this it would
  // keep reading files until it hit its own bound, on a machine that has already moved on.
  useEffect(() => cancelInFlight, [cancelInFlight]);

  return useMemo(
    () => ({ ...state, cancel: cancelInFlight, query, setQuery }),
    [cancelInFlight, query, state],
  );
}
