import { useEffect, useState } from "react";
import type { Goal } from "../contracts/goal";
import { goalService } from "../services/runtime-goal-client";
import { workBoardService } from "../services/runtime-work-board-client";
import type { WorkItem } from "../types/work-board";
import { selectRelatedGoals, selectRelatedWorkItems } from "./workspace-plan-links";

export interface WorkspacePlanLinks {
  workItems: WorkItem[];
  goals: Goal[];
}

/**
 * Fetches once per selected workspace id, over the same two existing services Work Board / Goal
 * Center already use for their own full lists (`workBoardService.listWorkItems`,
 * `goalService.listGoals`) -- no new Tauri command or service interface -- then joins client-side
 * via `workspace-plan-links.ts`'s exact-match rule. Matches `session-execution-context.ts`'s own
 * "pure join over an existing fetch" precedent (task 13.7's own brief cites it directly).
 *
 * Scoped to the *selected* workspace only, not the whole Projects list: fetching this for every
 * known workspace would fan out two extra requests per row for a section nobody may ever open --
 * the same reasoning `use-project-workspaces.ts` gives for keeping its own `inspectProject` scan
 * bounded to what is actually on screen. Re-fetches whenever `workspaceId` itself changes, not on
 * every render of the panel or every reload of the outer workspace list, and drops a response that
 * resolves after selection has already moved on to a different workspace.
 *
 * Only non-archived work items are treated as "related" here, matching Work Board's own default
 * (active) view -- an archived item is not something a read-only workspace detail panel should
 * present as current.
 */
export function useWorkspacePlanLinks(workspaceId: string | null) {
  const [data, setData] = useState<WorkspacePlanLinks | undefined>(undefined);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!workspaceId) {
      setData(undefined);
      setError(null);
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    void (async () => {
      try {
        const [workItems, goals] = await Promise.all([
          workBoardService.listWorkItems({ archived: false }),
          goalService.listGoals(),
        ]);
        if (cancelled) return;
        setData({ goals: selectRelatedGoals(goals, workspaceId), workItems: selectRelatedWorkItems(workItems, workspaceId) });
      } catch (reason: unknown) {
        if (!cancelled) setError(reason instanceof Error ? reason.message : String(reason));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [workspaceId]);

  return { data, error, loading };
}
