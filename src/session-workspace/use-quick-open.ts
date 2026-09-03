import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import { isCursorRefusal } from "./search-coverage";
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

/**
 * Distinguishes one dialog's searches from another's.
 *
 * Module scope rather than per hook. The native registry is keyed by search id alone, so a second
 * dialog reusing the first's id would supersede a search nobody replaced.
 */
let nextPanelId = 0;

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
 * One search id for the panel's whole life, reused for every keystroke. That is what makes the
 * newest request supersede the ones it replaced under the native registry's own lock — a walk on a
 * blocking thread does not stop because this side lost interest, and a held-down key would otherwise
 * put one thread per repeat behind answers nobody is waiting for.
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
  /**
   * The newest generation whose answer has been applied.
   *
   * The counter above knows what this hook asked for; the generation is what the native registry
   * actually ran, and a request whose predecessor is still winding down can have its answer arrive
   * second. Zero because the native counter starts at one.
   */
  const applied = useRef(0);
  const searchId = useRef<string | null>(null);
  if (searchId.current === null) searchId.current = `quick-open-${(nextPanelId += 1)}`;
  const panelSearchId = searchId.current;

  useEffect(() => {
    if (!isOpen) setQuery("");
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen || !sessionId) {
      setState(EMPTY);
      return;
    }
    const request = (issued.current += 1);
    applied.current = 0;
    setState((current) => ({ ...current, isLoading: true, failed: false }));
    const timer = setTimeout(() => {
      agentService
        .searchWorkspacePaths({ sessionId, query, searchId: panelSearchId })
        .then((result) => {
          // Dropped if anything newer has been issued. This is what "cancelled" means on this side:
          // the answer arrives, and nobody looks at it.
          if (issued.current !== request) return;
          if (result.generation < applied.current) return;
          applied.current = result.generation;
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
  }, [isOpen, panelSearchId, query, sessionId]);

  const loadMore = useCallback(() => {
    if (!sessionId || !state.nextCursor) return;
    const request = (issued.current += 1);
    const cursor = state.nextCursor;
    setState((current) => ({ ...current, isLoading: true }));
    agentService
      .searchWorkspacePaths({ sessionId, query, searchId: panelSearchId, cursor })
      .then((result) => {
        if (issued.current !== request) return;
        if (result.generation < applied.current) return;
        applied.current = result.generation;
        setState((current) => ({
          // Appended rather than replaced: a reader who asked for more expects the list to grow,
          // not to jump to a page they have to scroll back from.
          //
          // Except across a refused cursor. That page is not a continuation of this list — the
          // cursor named a rank in an ordering that no longer applies — so appending it would grow
          // the list with rows from a different result set. Replacing with nothing and dropping the
          // cursor ends the paging honestly, and the coverage says why.
          matches: isCursorRefusal(result.coverage) ? current.matches : [...current.matches, ...result.matches],
          coverage: result.coverage,
          nextCursor: isCursorRefusal(result.coverage) ? null : result.nextCursor ?? null,
          isLoading: false,
          failed: false,
        }));
      })
      .catch(() => {
        if (issued.current !== request) return;
        // The page already on screen is still true, so the failure only ends the paging.
        setState((current) => ({ ...current, isLoading: false, nextCursor: null }));
      });
  }, [panelSearchId, query, sessionId, state.nextCursor]);

  return useMemo(
    () => ({ ...state, loadMore, query, setQuery }),
    [loadMore, query, state],
  );
}
