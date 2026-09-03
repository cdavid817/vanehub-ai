import type { Dispatch } from "react";
import { agentService } from "../services/runtime-agent-client";
import type { KnownRemoteWorkspace, ProjectInspection } from "../types/agent";
import type { CreateSessionDraftAction } from "./create-session-draft-model";

/**
 * Task 13.9's own "validated workspace id": deliberately just an id and a kind, not a whole
 * `WorkspaceSummary` -- `projects/workspace-summary.ts` owns that shape and this file (part of the
 * wizard, not of Projects) has no reason to import it. `kind` mirrors that type's own
 * `"local" | "ssh"` vocabulary by value, not by importing it -- the two are structurally identical
 * string unions, which is all TypeScript needs for every call site across this boundary to
 * type-check, matching how `runs-destination.tsx` already declares its own local prop shapes for
 * lazy-loaded destinations rather than importing a sibling feature's types.
 */
export interface CreateSessionWorkspacePrefill {
  workspaceId: string;
  kind: "local" | "ssh";
}

/**
 * Finds the known remote workspace a validated `prefill.workspaceId` (a `KnownRemoteWorkspace.uri`)
 * refers to, re-checked against `knownRemoteWorkspaces` as it stood the moment the wizard's own
 * reference-data fetch resolved -- never trusted blindly. Returns `undefined` when `prefill` is
 * absent, is not an `"ssh"` prefill, or (a real, honest possibility: the remembered path could have
 * been removed from history between `Projects` rendering the action and this fetch resolving) no
 * longer appears in that list. Callers must treat "no match" as "nothing to prefill", the same way
 * `workspace-aggregation.ts`'s own `matchConnection` treats an unmatched SSH connection.
 */
export function findPrefillRemoteWorkspace(
  prefill: CreateSessionWorkspacePrefill | null | undefined,
  knownRemoteWorkspaces: KnownRemoteWorkspace[],
): KnownRemoteWorkspace | undefined {
  if (!prefill || prefill.kind !== "ssh") return undefined;
  return knownRemoteWorkspaces.find((remote) => remote.uri === prefill.workspaceId);
}

/**
 * Applies a matched remote workspace to the draft, dispatching the exact same actions
 * `create-session-remote-workspace-section.tsx`'s own `selectHistory` already dispatches when a
 * reader picks a "recent remote workspace" row by hand -- prefilling is that same, already-real
 * interaction driven programmatically, not a second implementation of it. `selectedSshConnectionId`
 * is deliberately left untouched (stays whatever `reset` set it to): a remembered remote path and a
 * saved connection profile are different concepts with no shared foreign key
 * (`workspace-aggregation.ts`'s own `matchConnection` comment), and `selectHistory` never links them
 * either -- inventing a link here that the manual picker does not make would be a new, undiscussed
 * behavior, not a faithful mirror of it.
 */
export function applyRemoteWorkspacePrefill(match: KnownRemoteWorkspace, dispatch: Dispatch<CreateSessionDraftAction>): void {
  dispatch({ type: "set-workspace-mode", mode: "remote" });
  dispatch({ type: "set-remote-host", value: match.host });
  dispatch({ type: "set-remote-port", value: String(match.port ?? 22) });
  dispatch({ type: "set-remote-user", value: match.user ?? "" });
  dispatch({ type: "set-remote-path", value: match.path });
  dispatch({ type: "set-remote-display-name", value: match.displayName });
}

/**
 * Local prefill's own equivalent of `applyRemoteWorkspacePrefill`: reproduces exactly what
 * clicking a "recent project" row already does (`LocalWorkspaceSection`'s own
 * `onClick={() => onInspectPath(project.path)}`) -- a live `inspectProject` call, not a blind
 * `set-project-path`, so a prefilled path that no longer resolves surfaces the same "missing" fact
 * a manual click would. Defined here rather than reusing `use-create-session-draft.ts`'s own
 * `inspectPath` closure: that closure also threads `patchLifecycle`'s error banner through, which
 * would pull a non-stable, component-scoped function into this effect's dependency array and force
 * it to re-run (and re-reset the whole draft) on every render. The one real difference from
 * `inspectPath` is this trade-off: an inspection failure here clears silently rather than raising
 * `lifecycle.error` -- the draft's `projectPath` is still set either way (the dispatch below is
 * unconditional), so Review remains reachable with the prefilled path shown either way, just
 * without a git/worktree annotation on it.
 */
export async function applyLocalWorkspacePrefill(
  path: string,
  dispatch: Dispatch<CreateSessionDraftAction>,
  setInspection: (inspection: ProjectInspection | null) => void,
): Promise<void> {
  dispatch({ type: "begin-project-path-inspection", path });
  setInspection(null);
  try {
    setInspection(await agentService.inspectProject(path));
  } catch {
    setInspection(null);
  }
}
