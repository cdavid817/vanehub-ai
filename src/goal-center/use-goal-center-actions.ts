import { useCallback, useEffect, useState } from "react";
import { goalService } from "../services/runtime-goal-client";
import type { DisplayableError } from "../ui/async/async-view-state";
import { useMutationRegistry } from "../ui/async/mutation-state";
import type { Goal, GoalInput, GoalLinkTarget } from "../contracts/goal";

/** Fixed registry key for the create-form's own in-flight request, matching
 *  use-work-board-actions.ts's `CREATE_MUTATION_KEY` precedent -- a not-yet-created goal has no
 *  id yet, and this string can never collide with a real goal id. */
export const CREATE_MUTATION_KEY = "goal-center:create";

function toDisplayableError(reason: unknown): DisplayableError {
  return { kind: "error", message: reason instanceof Error ? reason.message : String(reason), retryable: false };
}

/**
 * Per-goal mutation orchestration for the Goal Center (tasks 15.7-15.8).
 *
 * Replaces the previous single page-wide `busy` flag plus a full `load()` after every mutation
 * (goal-center.tsx's old `perform()`) with per-goal pending/error state from `useMutationRegistry`
 * (src/ui/async/mutation-state.ts, §3.14), keyed by goal id.
 *
 * Unlike the work board (use-work-board-actions.ts), every goal-returning action here --
 * create/update/link/unlink/activate/accept/reopen/abandon -- is request-then-reconcile-only: no
 * optimistic guess is applied to any of them. This is a deliberate choice, not the path of least
 * resistance: `Goal` carries several server-computed fields (`derivedStatus`, `counted`,
 * `terminal`, `unresolvable`, see contracts/goal.ts) that are recomputed from the goal's links on
 * every read, and even a self-contained action like `unlink` can change all four in ways the
 * client cannot predict from the target id alone (unlinking a stranded link changes
 * `unresolvable`, which can flip `derivedStatus`). `update` is the one action whose outcome *is*
 * safely predictable from its own input -- title/description/acceptanceNotes/projectPath do not
 * feed the derived fields at all, the same reasoning that makes the work board's own `update`
 * optimistic -- but it is kept reconcile-only anyway so every action in this hook follows one
 * uniform, easy-to-audit rule instead of a mix only one contributor is likely to remember
 * correctly. Goal Center's interactions are also discrete button clicks and form submits, not the
 * work board's continuous drag-and-drop, where instant visual feedback matters far more. Every
 * action still disables only its own goal's controls via `mutations` (Decision 11: "mutation 只
 * 禁用目标动作，保留其他内容"), so this is strictly a "no optimistic data" choice, not a return to
 * the page-wide `busy` flag it replaces.
 *
 * `deleteGoal` is the exception, and not a choice: it resolves to `void`, so there is no server
 * response to reconcile against. It is handled the same way the work board's own `remove`
 * special-cases delete -- optimistic removal, with the removed goal re-inserted at its original
 * index if the request is rejected. Goal Center's list has no rank/sort key (goal-presentation.ts
 * sorts nothing; the list renders `listGoals()`'s own order as-is), so re-inserting at the
 * captured index is what "preserve current order" means here.
 *
 * No version/revision field exists on `Goal` -- so, as with the work board, there is nothing to
 * wire for real optimistic-concurrency conflict detection. A rejected mutation still surfaces via
 * `mutations.fail`, it just cannot distinguish "a conflicting concurrent edit" from any other
 * failure reason.
 */
export function useGoalCenterActions() {
  const [goals, setGoals] = useState<Goal[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const mutations = useMutationRegistry();

  // Center-wide fetch: legitimate for initial mount only. Task 15.8 removes reload as the
  // automatic default after a single goal's own action, not this.
  const load = useCallback(async () => {
    setLoading(true);
    try {
      setGoals(await goalService.listGoals());
      setError(null);
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => { void load(); }, [load]);

  /** Marks `goalId` pending, calls the service, and patches just that one goal in `goals` from
   *  the server's own response -- see the hook doc comment for why nothing is guessed ahead of
   *  that response. */
  const mutateGoal = useCallback(async (
    goalId: string,
    call: () => Promise<Goal>,
    onSuccess?: (server: Goal) => void,
  ) => {
    mutations.begin(goalId);
    try {
      const server = await call();
      setGoals((current) => current.map((candidate) => (candidate.id === goalId ? server : candidate)));
      mutations.succeed(goalId);
      onSuccess?.(server);
    } catch (reason: unknown) {
      mutations.fail(goalId, toDisplayableError(reason));
    }
  }, [mutations]);

  const update = useCallback((goal: Goal, input: GoalInput, onSuccess?: () => void) =>
    mutateGoal(goal.id, () => goalService.updateGoal(goal.id, input), () => onSuccess?.()),
  [mutateGoal]);

  const link = useCallback((goal: Goal, targetKind: GoalLinkTarget, targetId: string) =>
    mutateGoal(goal.id, () => goalService.linkGoalTarget(goal.id, targetKind, targetId)),
  [mutateGoal]);

  const unlink = useCallback((goal: Goal, targetKind: GoalLinkTarget, targetId: string) =>
    mutateGoal(goal.id, () => goalService.unlinkGoalTarget(goal.id, targetKind, targetId)),
  [mutateGoal]);

  const activate = useCallback((goal: Goal) =>
    mutateGoal(goal.id, () => goalService.activateGoal(goal.id)), [mutateGoal]);

  const accept = useCallback((goal: Goal) =>
    mutateGoal(goal.id, () => goalService.acceptGoal(goal.id)), [mutateGoal]);

  const reopen = useCallback((goal: Goal) =>
    mutateGoal(goal.id, () => goalService.reopenGoal(goal.id)), [mutateGoal]);

  const abandon = useCallback((goal: Goal) =>
    mutateGoal(goal.id, () => goalService.abandonGoal(goal.id)), [mutateGoal]);

  // Delete has no server response to reconcile against, so it is handled outside mutateGoal: an
  // optimistic removal, with the pre-mutation goal re-inserted at its original index (there is no
  // rank/sort to restore instead -- see the hook doc comment) if the request is rejected.
  const remove = useCallback(async (goal: Goal, onSuccess?: () => void) => {
    mutations.begin(goal.id);
    const index = goals.findIndex((candidate) => candidate.id === goal.id);
    setGoals((current) => current.filter((candidate) => candidate.id !== goal.id));
    try {
      await goalService.deleteGoal(goal.id);
      mutations.succeed(goal.id);
      onSuccess?.();
    } catch (reason: unknown) {
      setGoals((current) => {
        if (current.some((candidate) => candidate.id === goal.id)) return current;
        const next = [...current];
        next.splice(Math.min(Math.max(index, 0), next.length), 0, goal);
        return next;
      });
      mutations.fail(goal.id, toDisplayableError(reason));
    }
  }, [goals, mutations]);

  // Not optimistic: the server assigns the id, timestamps, and every derived field, so there is
  // nothing safe to guess ahead of the response. Appended directly (no full reload) once the
  // response is known.
  const create = useCallback(async (input: GoalInput, onSuccess?: (goal: Goal) => void) => {
    mutations.begin(CREATE_MUTATION_KEY);
    try {
      const server = await goalService.createGoal(input);
      setGoals((current) => [...current, server]);
      mutations.succeed(CREATE_MUTATION_KEY);
      onSuccess?.(server);
    } catch (reason: unknown) {
      mutations.fail(CREATE_MUTATION_KEY, toDisplayableError(reason));
    }
  }, [mutations]);

  return { abandon, accept, activate, create, error, goals, link, load, loading, mutations, remove, reopen, unlink, update };
}
