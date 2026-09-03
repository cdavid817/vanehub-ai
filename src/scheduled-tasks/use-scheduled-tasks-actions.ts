import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { DisplayableError } from "../ui/async/async-view-state";
import { useMutationRegistry } from "../ui/async/mutation-state";
import type {
  CreateScheduledTaskInput, RunScheduledTaskNowResult, ScheduledTask, UpdateScheduledTaskInput,
} from "../types/agent";

/** Fixed registry key for the create-sheet's own in-flight request, matching
 *  `use-work-board-actions.ts`'s/`use-goal-center-actions.ts`'s own `CREATE_MUTATION_KEY`
 *  precedent -- a not-yet-created task has no id yet, and this string can never collide with a
 *  real task id. */
export const SCHEDULED_TASK_CREATE_MUTATION_KEY = "scheduled-tasks:create";

const VERSION_CONFLICT_PREFIX = "scheduled-task-version-conflict:";

/**
 * 19.8 established this as a stable code on *both* backends verbatim (Tauri's
 * `CommandError::typed(CommandErrorCategory::Conflict, ...)` in `scheduled_tasks.rs`'s
 * `version_conflict`; the Web mock's own literal `throw new Error(...)` in
 * `web-scheduled-task-client.ts`) -- unlike Loop Center's own `isLoopVersionConflict`
 * (loop-definition-dialog.tsx), which has to fall back to matching Tauri's locale-fixed
 * validation prose because that backend never adopted a stable code for its own conflict. A
 * plain prefix match is enough here; there is no fragile substring to maintain.
 */
export function isScheduledTaskVersionConflict(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return message.startsWith(VERSION_CONFLICT_PREFIX);
}

function toDisplayableError(reason: unknown): DisplayableError {
  return { kind: "error", message: reason instanceof Error ? reason.message : String(reason), retryable: false };
}

/**
 * Per-task mutation orchestration for the Scheduled Tasks panel (19.16-19.17), replacing the
 * previous single page-wide `error` state (`scheduled-tasks-panel.tsx`'s own pre-19.16 shape)
 * that funneled Create, Enable/Disable of *any* row, and Delete of *any* row into one shared
 * message shown only under the create form. That was a real grouping bug, not a stylistic
 * choice: an Enable/Disable failure on row B rendered nowhere near row B, with no visible
 * connection to the row that actually failed. Matches `use-goal-center-actions.ts`/
 * `use-work-board-actions.ts`'s own `useMutationRegistry` adoption, keyed by task id.
 *
 * Every one of a task's own mutating actions -- Enable/Disable, Delete, Update (Edit's own save),
 * and Run now -- share one registry slot per task id, the same "disable only this target's own
 * controls" granularity `goal-detail.tsx`'s own `pending` flag already uses for
 * Activate/Accept/Reopen/Edit/Delete/Abandon together. This is a deliberate choice: Run now
 * dispatching the task's *current* content/agent while an Edit save for the same task is still in
 * flight is a genuine race worth preventing in the UI. Create/Duplicate key off
 * `SCHEDULED_TASK_CREATE_MUTATION_KEY` instead, since a not-yet-created task cannot race any
 * existing row's own mutation (matches `loop-definition-overview.tsx`'s own doc comment: Duplicate
 * neither blocks nor is blocked by its source row's other actions).
 *
 * `create`/`update` return the server row (or throw) instead of taking an `onSuccess` callback
 * the way `setEnabled`/`remove` do: `ScheduledTaskEditorSheet` needs to `await` a save to run its
 * own version-conflict recovery (refetch, explain, let the reader retry -- see that component's
 * own doc comment) before deciding whether to close, which a fire-and-forget callback cannot
 * express. `setEnabled`/`remove`/`runNow` have no such follow-up decision to make, so they keep
 * the simpler callback style already established by `use-goal-center-actions.ts`.
 */
export function useScheduledTasksActions() {
  const { t } = useTranslation();
  const [tasks, setTasks] = useState<ScheduledTask[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const mutations = useMutationRegistry();

  const load = useCallback(async (): Promise<ScheduledTask[] | null> => {
    setLoading(true);
    try {
      const fetched = await agentService.listScheduledTasks();
      setTasks(fetched);
      setError(null);
      return fetched;
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      return null;
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => { void load(); }, [load]);

  const mutateTask = useCallback(async (
    taskId: string,
    call: () => Promise<ScheduledTask>,
    onSuccess?: (server: ScheduledTask) => void,
  ) => {
    mutations.begin(taskId);
    try {
      const server = await call();
      setTasks((current) => current.map((candidate) => (candidate.id === taskId ? server : candidate)));
      mutations.succeed(taskId);
      onSuccess?.(server);
    } catch (reason) {
      mutations.fail(taskId, toDisplayableError(reason));
    }
  }, [mutations]);

  const setEnabled = useCallback((task: ScheduledTask, enabled: boolean) =>
    mutateTask(task.id, () => agentService.setScheduledTaskEnabled({ enabled, taskId: task.id })),
  [mutateTask]);

  /** Reconciles `tasks` and this task's own registry slot exactly like `mutateTask`, but rethrows
   *  so `ScheduledTaskEditorSheet` can `await` it and run its own version-conflict recovery. A
   *  conflict fails with a translated, friendly message here (not the raw
   *  `scheduled-task-version-conflict: expected X, stored Y` string) since this row's own badge
   *  has no "let the reader retry" affordance of its own -- that lives in the still-open sheet,
   *  which shows its own more precise message (still-exists vs. deleted) independently. */
  const update = useCallback(async (task: ScheduledTask, input: UpdateScheduledTaskInput): Promise<ScheduledTask> => {
    mutations.begin(task.id);
    try {
      const server = await agentService.updateScheduledTask(input);
      setTasks((current) => current.map((candidate) => (candidate.id === task.id ? server : candidate)));
      mutations.succeed(task.id);
      return server;
    } catch (reason) {
      mutations.fail(task.id, isScheduledTaskVersionConflict(reason)
        ? { kind: "error", message: t("scheduledTasks.editor.versionConflict"), retryable: false }
        : toDisplayableError(reason));
      throw reason;
    }
  }, [mutations, t]);

  // Delete has no server response to reconcile against, so it is handled outside `mutateTask`:
  // an optimistic removal, with the pre-mutation task re-inserted at its original index (matching
  // `use-goal-center-actions.ts`'s own `remove`) if the request is rejected.
  const remove = useCallback(async (task: ScheduledTask, onSuccess?: () => void) => {
    mutations.begin(task.id);
    const index = tasks.findIndex((candidate) => candidate.id === task.id);
    setTasks((current) => current.filter((candidate) => candidate.id !== task.id));
    try {
      await agentService.deleteScheduledTask(task.id);
      mutations.succeed(task.id);
      onSuccess?.();
    } catch (reason) {
      setTasks((current) => {
        if (current.some((candidate) => candidate.id === task.id)) return current;
        const next = [...current];
        next.splice(Math.min(Math.max(index, 0), next.length), 0, task);
        return next;
      });
      mutations.fail(task.id, toDisplayableError(reason));
    }
  }, [tasks, mutations]);

  /** 19.10, unchanged in substance: a successful run does not change anything about the task
   *  itself (recurrence and latest-status stay the sweep's own bookkeeping), so there is no
   *  `tasks` update to make on success -- only this task's own mutation slot moves. */
  const runNow = useCallback(async (task: ScheduledTask): Promise<RunScheduledTaskNowResult | null> => {
    mutations.begin(task.id);
    try {
      const result = await agentService.runScheduledTaskNow(task.id);
      mutations.succeed(task.id);
      return result;
    } catch (reason) {
      mutations.fail(task.id, toDisplayableError(reason));
      return null;
    }
  }, [mutations]);

  // Not optimistic: the server assigns the id, timestamps, and computed `nextRunAt`, so there is
  // nothing safe to guess ahead of the response. 19.9's Duplicate reaches this exact function too
  // (through the same editor sheet in create mode) -- there is no separate duplicate-specific
  // service call, so a duplicated task never inherits `scheduled_task_runs` history: it simply
  // never existed as a row until this call creates a brand-new one.
  const create = useCallback(async (input: CreateScheduledTaskInput): Promise<ScheduledTask> => {
    mutations.begin(SCHEDULED_TASK_CREATE_MUTATION_KEY);
    try {
      const server = await agentService.createScheduledTask(input);
      setTasks((current) => [server, ...current]);
      mutations.succeed(SCHEDULED_TASK_CREATE_MUTATION_KEY);
      return server;
    } catch (reason) {
      mutations.fail(SCHEDULED_TASK_CREATE_MUTATION_KEY, toDisplayableError(reason));
      throw reason;
    }
  }, [mutations]);

  return { create, error, load, loading, mutations, remove, runNow, setEnabled, tasks, update };
}
