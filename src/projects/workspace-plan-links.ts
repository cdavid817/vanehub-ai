import type { Goal } from "../contracts/goal";
import type { WorkItem } from "../types/work-board";

/**
 * Both `WorkItem.projectPath` and `Goal.projectPath` are plain free-text fields (see
 * `work-board-form.tsx`/`goal-form.tsx`'s own `<input>` for `projectPath` -- no folder picker, no
 * format validation beyond trim-to-null), unlike `Session`, which has dedicated
 * `remoteWorkspace`/`remoteSshConnectionId` fields specifically for its SSH case. Nothing in
 * either type restricts `projectPath` to local filesystem paths, so this join is applied by exact
 * string equality against `WorkspaceSummary.workspaceId` regardless of workspace kind -- it is
 * not special-cased to local rows the way `workspace-aggregation.ts`'s own Session join is. In
 * practice this only ever matches local rows today (nothing currently writes an `ssh://` URI into
 * either form), but that is a fact about how the forms happen to be used, not a rule this join
 * enforces -- see `use-workspace-plan-links.ts` for where the underlying lists come from.
 */
export function selectRelatedWorkItems(workItems: WorkItem[], workspaceId: string): WorkItem[] {
  return workItems.filter((item) => item.projectPath === workspaceId);
}

export function selectRelatedGoals(goals: Goal[], workspaceId: string): Goal[] {
  return goals.filter((goal) => goal.projectPath === workspaceId);
}
