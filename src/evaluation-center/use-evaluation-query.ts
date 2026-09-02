import { useEffect, useState, type Dispatch, type SetStateAction } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { AgentRegistryEntry } from "../types/agent";
import type { EvaluationArena, EvaluationOutcome, EvaluationTask } from "../types/evaluation";

/** Outcomes that will never change again on their own -- the polling effect below stops (and the
 *  detail pane hides Cancel) once every attempt in every arena has landed in one of these. */
export const TERMINAL_EVALUATION_OUTCOMES = new Set<EvaluationOutcome>([
  "succeeded", "task_failed", "agent_failed", "timed_out", "stuck", "cancelled", "benchmark_error",
]);

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
}

/**
 * 18.2 "query model" extraction: the Agent/task catalog fetch, the arena history fetch, and the
 * visibility-aware polling reconciliation effect that used to live directly in
 * `evaluation-center.tsx`. Mirrors this same OpenSpec change's established "extract orchestration
 * into a `use-*` hook" pattern (`use-cli-management-actions.ts`, `use-mcp-test-operation.ts`,
 * `use-conversation-window-model.ts`) -- state the page and its child components read, owned in
 * one place instead of split across separate `useState` calls plus inline effects.
 *
 * Behavior is unchanged from the pre-extraction page: existing `evaluation-center.test.tsx` cases
 * (initial load success/failure, polling pause on hidden/resume on visible, polling stop on
 * unmount, following a polled arena instead of a stale captured attempt) already exercise every
 * path this hook owns, end to end through the composed page -- see that file rather than a
 * duplicate isolated suite here.
 */
export function useEvaluationQuery(): UseEvaluationQueryResult {
  const { t } = useTranslation();
  const [agents, setAgents] = useState<AgentRegistryEntry[]>([]);
  const [tasks, setTasks] = useState<EvaluationTask[]>([]);
  const [arenas, setArenas] = useState<EvaluationArena[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function loadInitial() {
      try {
        const [registry, catalog, history] = await Promise.all([
          agentService.listAgents(), agentService.listEvaluationTasks(), agentService.listEvaluationArenas(),
        ]);
        setAgents(registry); setTasks(catalog); setArenas(history);
      } catch { setError(t("evaluation.loadError")); }
    }
    void loadInitial();
  }, [t]);

  useEffect(() => {
    if (!arenas.some((arena) => arena.attempts.some((attempt) => !TERMINAL_EVALUATION_OUTCOMES.has(attempt.outcome)))) return;
    // design.md Decision 6: polling adjusts for document visibility, matching mission-control.tsx's
    // reconcile guard — a backgrounded tab skips the fetch, and regaining focus/visibility catches
    // up immediately rather than waiting out the rest of the interval.
    const reconcile = () => { if (document.visibilityState === "visible") void agentService.listEvaluationArenas().then(setArenas); };
    const timer = window.setInterval(reconcile, 1_000);
    window.addEventListener("focus", reconcile);
    document.addEventListener("visibilitychange", reconcile);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", reconcile);
      document.removeEventListener("visibilitychange", reconcile);
    };
  }, [arenas]);

  return { agents, arenas, error, setArenas, setError, tasks };
}
