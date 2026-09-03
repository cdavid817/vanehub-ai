import { useCallback, useEffect, useState } from "react";
import { workBoardService } from "../services/runtime-work-board-client";
import type { DisplayableError } from "../ui/async/async-view-state";
import { useMutationRegistry } from "../ui/async/mutation-state";
import type { WorkItem, WorkItemStage } from "../types/work-board";
import type { WorkItemFormValues } from "./work-board-form";

/** Fixed registry key for the create-form's own in-flight request. A not-yet-created item has no
 *  id yet, and this string can never collide with a real work item id. */
export const CREATE_MUTATION_KEY = "work-board:create";

function toDisplayableError(reason: unknown): DisplayableError {
  // Matches settings' saveSetting (12.10) precedent: rollback already restores the pre-mutation
  // value into the UI, so redoing the same action *is* the retry -- there is no separately cached
  // "last attempted value" a retry button would need to replay.
  return { kind: "error", message: reason instanceof Error ? reason.message : String(reason), retryable: false };
}

/**
 * Per-card mutation orchestration for the work board (tasks 14.10-14.11).
 *
 * Replaces the previous single page-wide `busy` flag plus a full `load()` after every mutation
 * with per-card pending/error state from `useMutationRegistry` (src/ui/async/mutation-state.ts,
 * §3.14), keyed by work item id so several cards can each have their own mutation in flight at
 * once. This is a deliberate choice, not the default: SSH/MCP/CLI Management's own settings pages
 * each evaluated this same registry and declined it in favor of projecting a single `useMutation`
 * call's state, because each of those pages only ever has one in-flight mutation of a given kind
 * at a time (react-query's own `variables`/`isPending` already tracks that fact, and a registry
 * alongside it would just be a second source of truth -- see tasks.md §12.18/§12.19 notes). The
 * work board has no such single-flight constraint: dragging one card to a new stage while another
 * card's edit is still saving is routine, independent, concurrent use, which is exactly the case
 * this registry exists for. This hook is this codebase's first real production caller of it.
 *
 * No version/revision field exists on `WorkItem` today (TypeScript type, Rust model, or the web
 * mock) -- so there is nothing to wire for real optimistic-concurrency conflict detection. A
 * rejected mutation still rolls back correctly (see `mutateCard`), it just cannot distinguish "a
 * conflicting concurrent edit" from any other failure reason.
 */
export function useWorkBoardActions(archived: boolean) {
  const [items, setItems] = useState<WorkItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const mutations = useMutationRegistry();

  // Board-wide fetch: legitimate for initial mount and for switching the active/archived view (a
  // genuinely different dataset from the server, not a reload triggered by one card's own
  // mutation). Task 14.11 removes reload as the *automatic default* after a single card's own
  // action, not this.
  const load = useCallback(async () => {
    setLoading(true);
    try {
      setItems(await workBoardService.listWorkItems({ archived }));
      setError(null);
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }, [archived]);
  useEffect(() => { void load(); }, [load]);

  /** Optimistically applies `optimistic` for `item`, calls the service, reconciles the affected
   *  card to the server's own response, and rolls back to the pre-mutation `item` on failure.
   *  Rollback only ever touches this one card -- other cards may have their own concurrent
   *  optimistic mutations in flight, and restoring a full pre-mutation list snapshot would
   *  clobber those too (settings' saveSetting has a documented, accepted version of this same
   *  race for its single settings object; a per-card list needs the narrower fix). If the
   *  server's response no longer belongs to the current `archived` scope (e.g. an archive/restore
   *  crossing the active/archived boundary), the card is dropped from `items` instead of patched,
   *  since a fresh fetch of this scope would no longer return it either.
   *
   *  Returns whether the mutation ultimately succeeded. This never throws -- rollback already
   *  happens internally, and a successful mutation's own registry entry is deleted rather than
   *  kept as a tombstone (`mutations.succeed`) -- so the boolean return is the only way a caller
   *  that runs several of these concurrently (14.12's batch mode, `use-work-board-batch.ts`) can
   *  learn a given item's own outcome without racing a stale closure over `mutations.registry`. */
  const mutateCard = useCallback(async (
    item: WorkItem,
    optimistic: WorkItem,
    call: () => Promise<WorkItem>,
    onSuccess?: () => void,
  ): Promise<boolean> => {
    mutations.begin(item.id);
    setItems((current) => current.map((candidate) => (candidate.id === item.id ? optimistic : candidate)));
    try {
      const server = await call();
      setItems((current) => (server.archived === archived
        ? current.map((candidate) => (candidate.id === item.id ? server : candidate))
        : current.filter((candidate) => candidate.id !== item.id)));
      mutations.succeed(item.id);
      onSuccess?.();
      return true;
    } catch (reason: unknown) {
      setItems((current) => current.map((candidate) => (candidate.id === item.id ? item : candidate)));
      mutations.fail(item.id, toDisplayableError(reason));
      return false;
    }
  }, [archived, mutations]);

  // Stage is user-chosen, so guessing the outcome is safe; the server-computed `rank` is
  // reconciled in once `call()` resolves.
  const move = useCallback((item: WorkItem, stage: WorkItemStage) =>
    mutateCard(item, { ...item, stage }, () => workBoardService.moveWorkItem({ workItemId: item.id, stage })),
  [mutateCard]);

  const archive = useCallback((item: WorkItem) =>
    mutateCard(item, { ...item, archived: true }, () => workBoardService.archiveWorkItem(item.id)),
  [mutateCard]);

  const restore = useCallback((item: WorkItem) =>
    mutateCard(item, { ...item, archived: false }, () => workBoardService.restoreWorkItem(item.id)),
  [mutateCard]);

  // Every field in WorkItemFormValues is exactly what the user submitted, so it is a safe guess.
  const update = useCallback((item: WorkItem, input: WorkItemFormValues, onSuccess?: () => void) =>
    mutateCard(item, { ...item, ...input }, () => workBoardService.updateWorkItem({ workItemId: item.id, ...input }), onSuccess),
  [mutateCard]);

  // Delete has no server response to reconcile against, so it is handled outside mutateCard: an
  // optimistic removal, with the pre-mutation item re-inserted (order restored by the rank sort
  // the board already applies) if the request is rejected.
  const remove = useCallback(async (item: WorkItem) => {
    mutations.begin(item.id);
    setItems((current) => current.filter((candidate) => candidate.id !== item.id));
    try {
      await workBoardService.deleteWorkItem(item.id);
      mutations.succeed(item.id);
    } catch (reason: unknown) {
      setItems((current) => (current.some((candidate) => candidate.id === item.id) ? current : [...current, item]));
      mutations.fail(item.id, toDisplayableError(reason));
    }
  }, [mutations]);

  // Not optimistic: the server assigns the id, rank, and timestamps, so there is nothing safe to
  // guess ahead of the response. Appended directly (no full reload) once the response is known,
  // and only if it still belongs to the currently loaded (active/archived) scope.
  const create = useCallback(async (input: WorkItemFormValues, onSuccess?: () => void) => {
    mutations.begin(CREATE_MUTATION_KEY);
    try {
      const server = await workBoardService.createWorkItem(input);
      if (server.archived === archived) setItems((current) => [...current, server]);
      mutations.succeed(CREATE_MUTATION_KEY);
      onSuccess?.();
    } catch (reason: unknown) {
      mutations.fail(CREATE_MUTATION_KEY, toDisplayableError(reason));
    }
  }, [archived, mutations]);

  return { archive, create, error, items, load, loading, move, mutations, remove, restore, update };
}
