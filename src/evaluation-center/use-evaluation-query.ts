import { useCallback, useEffect, useState, type Dispatch, type SetStateAction } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { AgentRegistryEntry } from "../types/agent";
import type { EvaluationArena, EvaluationOutcome, EvaluationTask } from "../types/evaluation";

/** Outcomes that will never change again on their own -- the polling effect below stops (and the
 *  detail pane hides Cancel) once every attempt in every arena has landed in one of these. */
export const TERMINAL_EVALUATION_OUTCOMES = new Set<EvaluationOutcome>([
  "succeeded", "task_failed", "agent_failed", "timed_out", "stuck", "cancelled", "benchmark_error",
]);

// 18.6: bounds how many already-loaded arenas one reconcile tick refreshes in a single request.
// Matches the service layer's own per-request ceiling (`list_evaluation_arenas.rs`'s and
// `web-evaluation-client.ts`'s own identical `MAX_LIMIT`/`MAX_EVALUATION_PAGE_LIMIT` = 50) -- arenas
// sort newest-first, so a still-in-flight attempt is overwhelmingly likely to be in this window.
// Anything a reader reached only via `loadMoreArenas` beyond it keeps its last-known state until
// the next tick brings it back into the window or a full reload happens. A known, bounded tradeoff
// documented in this task's own tasks.md evidence, not a silent one.
const RECONCILE_LIMIT = 50;

export interface UseEvaluationQueryResult {
  agents: AgentRegistryEntry[];
  tasks: EvaluationTask[];
  arenas: EvaluationArena[];
  /**
   * Exposed directly rather than wrapped: start/cancel/export stay page-level orchestration in
   * this pass (task 18.2's "query model" half only covers the fetch and the polling
   * reconciliation), and the page still decides *when* and *how* arenas/error change -- prepend a
   * fresh arena, replace one by id after a cancel, clear an error on retry. This hook only owns
   * *where* that state lives.
   */
  setArenas: Dispatch<SetStateAction<EvaluationArena[]>>;
  error: string | null;
  setError: Dispatch<SetStateAction<string | null>>;
  /** 18.6: whether the service reports a further page beyond what `arenas` currently holds. */
  hasMoreArenas: boolean;
  /** True only while a `loadMoreArenas()` request is in flight, so the list can disable its own
   *  "load more" control instead of letting a reader queue up duplicate page requests. */
  loadingMoreArenas: boolean;
  /** No-op while there is no further page or a request is already in flight. Appends the next
   *  page, de-duplicated by id against what is already loaded -- offset pagination over a list a
   *  new arena can prepend to can otherwise hand back one item the reader already has (see this
   *  task's own tasks.md evidence for the exact scenario). */
  loadMoreArenas: () => void;
}

/**
 * 18.2 "query model" extraction: the Agent/task catalog fetch, the arena history fetch, and the
 * visibility-aware polling reconciliation effect that used to live directly in
 * `evaluation-center.tsx`. Mirrors this same OpenSpec change's established "extract orchestration
 * into a `use-*` hook" pattern (`use-cli-management-actions.ts`, `use-mcp-test-operation.ts`,
 * `use-conversation-window-model.ts`) -- state the page and its child components read, owned in
 * one place instead of split across separate `useState` calls plus inline effects.
 *
 * 18.6 added real pagination on top of the above (`EvaluationService.listEvaluationArenas` now
 * takes a cursor/limit query and returns a page instead of the whole history): the initial load
 * fetches page one, `loadMoreArenas` appends subsequent pages, and the reconcile poll refreshes
 * only the newest `RECONCILE_LIMIT` arenas in place rather than replacing the whole array -- see
 * that constant's own comment for why a full-array replace would have silently dropped whatever
 * `loadMoreArenas` had appended on the very next poll tick.
 */
export function useEvaluationQuery(): UseEvaluationQueryResult {
  const { t } = useTranslation();
  const [agents, setAgents] = useState<AgentRegistryEntry[]>([]);
  const [tasks, setTasks] = useState<EvaluationTask[]>([]);
  const [arenas, setArenas] = useState<EvaluationArena[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [loadingMoreArenas, setLoadingMoreArenas] = useState(false);

  useEffect(() => {
    async function loadInitial() {
      try {
        const [registry, catalog, page] = await Promise.all([
          agentService.listAgents(), agentService.listEvaluationTasks(), agentService.listEvaluationArenas(),
        ]);
        setAgents(registry); setTasks(catalog); setArenas(page.items); setNextCursor(page.nextCursor);
      } catch { setError(t("evaluation.loadError")); }
    }
    void loadInitial();
  }, [t]);

  useEffect(() => {
    if (!arenas.some((arena) => arena.attempts.some((attempt) => !TERMINAL_EVALUATION_OUTCOMES.has(attempt.outcome)))) return;
    // design.md Decision 6: polling adjusts for document visibility, matching mission-control.tsx's
    // reconcile guard — a backgrounded tab skips the fetch, and regaining focus/visibility catches
    // up immediately rather than waiting out the rest of the interval.
    const reconcile = () => {
      if (document.visibilityState !== "visible") return;
      // Merges by id instead of replacing outright: `page.items` is authoritative for the newest
      // `RECONCILE_LIMIT` arenas (including one created elsewhere since the last tick), and
      // whatever `loadMoreArenas` appended beyond that window is kept as-is rather than dropped.
      // Pagination bookkeeping (`nextCursor`) is untouched here -- owned only by the initial load
      // and `loadMoreArenas`, since this fixed-size window rarely matches the reader's true paging
      // position once they have loaded more than one page.
      void agentService.listEvaluationArenas({ limit: RECONCILE_LIMIT }).then((page) => {
        setArenas((current) => {
          const fresh = new Set(page.items.map((item) => item.id));
          return [...page.items, ...current.filter((item) => !fresh.has(item.id))];
        });
      });
    };
    const timer = window.setInterval(reconcile, 1_000);
    window.addEventListener("focus", reconcile);
    document.addEventListener("visibilitychange", reconcile);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", reconcile);
      document.removeEventListener("visibilitychange", reconcile);
    };
  }, [arenas]);

  const loadMoreArenas = useCallback(() => {
    if (!nextCursor || loadingMoreArenas) return;
    setLoadingMoreArenas(true);
    void agentService.listEvaluationArenas({ cursor: nextCursor })
      .then((page) => {
        setArenas((current) => {
          const seen = new Set(current.map((item) => item.id));
          return [...current, ...page.items.filter((item) => !seen.has(item.id))];
        });
        setNextCursor(page.nextCursor);
      })
      .catch(() => { setError(t("evaluation.loadError")); })
      .finally(() => { setLoadingMoreArenas(false); });
  }, [loadingMoreArenas, nextCursor, t]);

  return { agents, arenas, error, hasMoreArenas: nextCursor !== null, loadingMoreArenas, loadMoreArenas, setArenas, setError, tasks };
}
