import { sshConnectionService } from "../services/runtime-ssh-connection-client";
import type { DisplayableError } from "../ui/async/async-view-state";
import { useMutationRegistry, type MutationRegistryApi } from "../ui/async/mutation-state";

// Same local shape as every other `use-*-actions.ts` in this codebase (goal-center,
// mission-control, work-board) -- not a shared utility, each feature owns its own copy rather than
// import one across a feature boundary for a two-line function.
function toDisplayableError(reason: unknown): DisplayableError {
  return { kind: "error", message: reason instanceof Error ? reason.message : String(reason), retryable: false };
}

export interface WorkspaceReconnectApi {
  mutations: MutationRegistryApi;
  reconnect: (workspaceId: string, connectionId: string) => Promise<void>;
}

/**
 * Task 13.8's Reconnect action, matched-SSH-connection case only: re-runs the exact same
 * `SshConnectionService.testConnection` call `ssh-connection-card.tsx`'s own "Test" action already
 * uses in Settings -- "reconnect" for a known workspace and "test" for its saved connection profile
 * are the same real operation seen from two different pages, not two different capabilities.
 *
 * Keyed by `workspaceId` rather than `connectionId` in the mutation registry: `Projects`' own
 * master-detail selection is by `workspaceId` (`workspace-card.tsx`'s de-facto unique key), so
 * looking a pending/error state up by that same id is what makes switching the selected workspace
 * naturally show that row's own state, mirroring `use-goal-center-actions.ts`'s per-goal keying.
 *
 * `onReconnected` is called only after a successful test, not after every attempt: a failed test
 * changes nothing about the connection worth re-fetching the workspace list for, and re-fetching on
 * every failure would also race the mutation registry's own error state with the reload's loading
 * state for no benefit.
 */
export function useWorkspaceReconnect(onReconnected: () => void): WorkspaceReconnectApi {
  const mutations = useMutationRegistry();

  async function reconnect(workspaceId: string, connectionId: string) {
    mutations.begin(workspaceId);
    try {
      await sshConnectionService.testConnection(connectionId);
      mutations.succeed(workspaceId);
      onReconnected();
    } catch (reason) {
      mutations.fail(workspaceId, toDisplayableError(reason));
    }
  }

  return { mutations, reconnect };
}
