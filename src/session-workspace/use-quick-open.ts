import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import type {
  WorkspacePathMatch,
  WorkspaceSearchCoverage,
} from "../types/session-workspace-inspection";

/**
 * How long a keystroke waits before it becomes a search.
 *
 * A workspace walk is not free, and a reader typing `components` would otherwise start ten of them.
 * Short enough that the list still feels like it is following the keyboard.
 */
const KEYSTROKE_DELAY_MS = 120;

interface QuickOpenState {
  matches: WorkspacePathMatch[];
  coverage: WorkspaceSearchCoverage | null;
  nextCursor: string | null;
  isLoading: boolean;
  failed: boolean;
}

const EMPTY: QuickOpenState = {
  matches: [],
  coverage: null,
  nextCursor: null,
  isLoading: false,
  failed: false,
};

/**
 * Quick Open's search, with the answers to abandoned keystrokes thrown away.
 *
 * Not a React Query, and the reason is the interesting part. Every keystroke is a different key, so
 * a cache would fill with one entry per prefix of everything anybody ever typed — and none of them
 * would ever be read again, because a reader types forwards. What this needs instead is the
 * opposite of caching: keep the newest answer and discard the rest.
 *
 * Cancellation is a frontend fact. A Tauri command is one round trip with no way to call it back,
 * so what "cancel" can mean here is that a superseded answer is never rendered. The bounded walk on
 * the native side finishes either way, which is why the bound is what actually protects the
 * machine — a cancellation token would be a mechanism for stopping something that stops anyway.
 */
export function useQuickOpen(sessionId: string | null, isOpen: boolean) {
  const [query, setQuery] = useState("");
  const [state, setState] = useState<QuickOpenState>(EMPTY);
  /**
   * Which request the rendered answer belongs to.
   *
   * A counter rather than comparing the query text: a reader who types `ma`, deletes to `m`, and
   * types `ma` again has issued three requests, two of which carry the same query. Matching on text
   * would let the first one's answer overwrite the third's.
   */
  const issued = useRef(0);

  useEffect(() => {
    if (!isOpen) setQuery("");
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen || !sessionId) {
      setState(EMPTY);
      return;
    }
    const request = (issued.current += 1);
    setState((current) => ({ ...current, isLoading: true, failed: false }));
    const timer = setTimeout(() => {
      agentService
        .searchWorkspacePaths({ sessionId, query })
        .then((result) => {
          // Dropped if anything newer has been issued. This is what "cancelled" means on this side:
          // the answer arrives, and nobody looks at it.
          if (issued.current !== request) return;
          setState({
            matches: result.matches,
            coverage: result.coverage,
            nextCursor: result.nextCursor ?? null,
            isLoading: false,
            failed: false,
          });
        })
        .catch(() => {
          if (issued.current !== request) return;
          setState({ ...EMPTY, failed: true });
        });
    }, KEYSTROKE_DELAY_MS);

    return () => {
      clearTimeout(timer);
    };
  }, [isOpen, query, sessionId]);

  const loadMore = useCallback(() => {
    if (!sessionId || !state.nextCursor) return;
    const request = (issued.current += 1);
    const cursor = state.nextCursor;
    setState((current) => ({ ...current, isLoading: true }));
    agentService
      .searchWorkspacePaths({ sessionId, query, cursor })
      .then((result) => {
        if (issued.current !== request) return;
        setState((current) => ({
          // Appended rather than replaced: a reader who asked for more expects the list to grow,
          // not to jump to a page they have to scroll back from.
          matches: [...current.matches, ...result.matches],
          coverage: result.coverage,
          nextCursor: result.nextCursor ?? null,
          isLoading: false,
          failed: false,
        }));
      })
      .catch(() => {
        if (issued.current !== request) return;
        // The page already on screen is still true, so the failure only ends the paging.
        setState((current) => ({ ...current, isLoading: false, nextCursor: null }));
      });
  }, [query, sessionId, state.nextCursor]);

  return useMemo(
    () => ({ ...state, loadMore, query, setQuery }),
    [loadMore, query, state],
  );
}
