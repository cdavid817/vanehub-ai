import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import type { AgentService } from "../../../services/agent-service";
import type { PersonalizationPolicyRef } from "../../../types/personalization";
import {
  beginSave,
  canSave,
  discardDraft,
  draftFromPolicy,
  editDraft,
  isDirty,
  keepMine,
  mergePolicies,
  saveConflicted,
  saveFailed,
  saveSucceeded,
  scopeKeyOf,
  takeTheirs,
  type InstructionDraftMap,
  type InstructionValues,
} from "./instruction-drafts";
import { isIncomplete } from "./scope-selector";

/** The code the native command layer sends; the message is the whole contract across the wire. */
const CONFLICT_CODE = "personalization-revision-conflict";

/** Everything a save needs, captured when it starts so a later scope change cannot rewrite it. */
interface SaveRequest {
  scope: PersonalizationPolicyRef;
  values: InstructionValues;
  expectedRevision: number | undefined;
}

export function policyQueryKey(scope: PersonalizationPolicyRef) {
  return ["personalization", "policy", scopeKeyOf(scope)] as const;
}

/**
 * Binds the draft engine to the service for one selected scope.
 *
 * Drafts for every scope stay in one map rather than resetting on selection, so switching layers
 * and coming back returns the user to what they were typing. That is also what makes in-app
 * navigation safe, and why the only moment work can be lost is leaving the window.
 */
export function useInstructionDrafts(service: AgentService, scope: PersonalizationPolicyRef) {
  const [drafts, setDrafts] = useState<InstructionDraftMap>({});
  const queryClient = useQueryClient();
  const incomplete = isIncomplete(scope);
  const key = scopeKeyOf(scope);

  const policyQuery = useQuery({
    queryKey: policyQueryKey(scope),
    queryFn: () => service.getPersonalizationPolicy(scope),
    enabled: !incomplete,
  });

  const loaded = policyQuery.isSuccess ? (policyQuery.data ?? null) : undefined;
  useEffect(() => {
    if (loaded === undefined) return;
    // `mergePolicies` decides what happens to a draft that is already open: a clean one follows the
    // store, a dirty one becomes a conflict rather than being overwritten.
    setDrafts((current) => mergePolicies(current, [{ scope, policy: loaded }]));
    // The scope is captured by its key: two different objects describing the same layer must not
    // re-merge and clear a conflict the user has not answered yet.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loaded, key]);

  const draft = drafts[key];

  const saveMutation = useMutation({
    mutationFn: async ({ expectedRevision, scope: target, values }: SaveRequest) =>
      service.patchPersonalizationPolicy({
        ...target,
        expectedRevision,
        aboutUser: values.aboutUser,
        styleRules: values.styleRules,
        instructionMergeMode: values.instructionMergeMode,
      }),
    // Every callback answers the scope the save was *started* for, taken from the request rather
    // than from the current selection. Switching layers mid-save is ordinary, and reading the
    // selection here would land one layer's result -- or its failure -- on whichever layer the
    // user happened to be looking at when the response arrived.
    onMutate: ({ scope: target }) => setDrafts((current) => beginSave(current, target)),
    onSuccess: (policy, { scope: target }) => {
      setDrafts((current) => saveSucceeded(current, target, policy));
      queryClient.setQueryData(policyQueryKey(target), policy);
      void queryClient.invalidateQueries({ queryKey: ["personalization", "overview"] });
      void queryClient.invalidateQueries({ queryKey: ["personalization", "policies"] });
    },
    onError: async (error: Error, { scope: target }) => {
      if (!error.message.includes(CONFLICT_CODE)) {
        setDrafts((current) => saveFailed(current, target, error.message));
        return;
      }
      // Read the store rather than trusting the numbers in the message: the user is about to be
      // shown what they would be overwriting, and that has to be the text, not just a revision.
      const stored = await service.getPersonalizationPolicy(target).catch(() => null);
      queryClient.setQueryData(policyQueryKey(target), stored);
      setDrafts((current) => saveConflicted(current, target, stored));
    },
  });

  // The repository has no shared unsaved-change guard, so this is the guard: the browser's own
  // prompt, armed only while something is dirty anywhere. In-app navigation is covered by the
  // drafts surviving it, which is why leaving the window is the only moment work can be lost.
  const dirtyCount = Object.values(drafts).filter(isDirty).length;
  // Task 12.16: same aggregate shape as `dirtyCount` above -- a failed save's `.error` (set by
  // `saveFailed`) survives on its own draft regardless of which scope is on screen right now.
  const hasError = Object.values(drafts).some((entry) => entry.error !== null);
  useEffect(() => {
    if (dirtyCount === 0) return;
    const warn = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirtyCount]);

  return {
    draft,
    dirtyCount,
    hasError,
    isDirty: draft ? isDirty(draft) : false,
    canSave: draft ? canSave(draft) : false,
    isLoading: !incomplete && policyQuery.isPending,
    loadError: policyQuery.error,
    edit: (patch: Partial<InstructionValues>) =>
      setDrafts((current) => editDraft(current, scope, patch)),
    discard: () => setDrafts((current) => discardDraft(current, scope)),
    save: () => {
      const current = drafts[key];
      if (!current || !canSave(current)) return;
      saveMutation.mutate({
        scope,
        values: current.values,
        // Absent creates the layer. Sending 0 would claim the caller saw a revision that does not
        // exist, and the store would refuse a first save forever.
        expectedRevision: current.baseRevision > 0 ? current.baseRevision : undefined,
      });
    },
    keepMine: () => setDrafts((current) => keepMine(current, scope)),
    /** Re-reads the stored side. `mergePolicies` then restates the conflict against fresh text,
     * because a conflict left open while someone else keeps editing is otherwise answered against
     * a snapshot taken when the save was refused. */
    reload: () => void queryClient.invalidateQueries({ queryKey: policyQueryKey(scope) }),
    takeTheirs: () => setDrafts((current) => takeTheirs(current, scope)),
    /** Only for a scope the caller knows is complete; a missing draft renders as still loading. */
    seed: () => setDrafts((current) => ({ ...current, [key]: draftFromPolicy(scope, null) })),
  };
}
