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
    mutationFn: async (values: InstructionValues) => {
      const base = drafts[key];
      return service.patchPersonalizationPolicy({
        ...scope,
        // Absent creates the layer. Sending 0 would claim the caller saw a revision that does not
        // exist, and the store would refuse a first save forever.
        expectedRevision: base && base.baseRevision > 0 ? base.baseRevision : undefined,
        aboutUser: values.aboutUser,
        styleRules: values.styleRules,
        instructionMergeMode: values.instructionMergeMode,
      });
    },
    onMutate: () => setDrafts((current) => beginSave(current, scope)),
    onSuccess: (policy) => {
      setDrafts((current) => saveSucceeded(current, scope, policy));
      queryClient.setQueryData(policyQueryKey(scope), policy);
      void queryClient.invalidateQueries({ queryKey: ["personalization", "overview"] });
      void queryClient.invalidateQueries({ queryKey: ["personalization", "policies"] });
    },
    onError: async (error: Error) => {
      if (!error.message.includes(CONFLICT_CODE)) {
        setDrafts((current) => saveFailed(current, scope, error.message));
        return;
      }
      // Read the store rather than trusting the numbers in the message: the user is about to be
      // shown what they would be overwriting, and that has to be the text, not just a revision.
      const stored = await service.getPersonalizationPolicy(scope).catch(() => null);
      queryClient.setQueryData(policyQueryKey(scope), stored);
      setDrafts((current) => saveConflicted(current, scope, stored));
    },
  });

  // The repository has no shared unsaved-change guard, so this is the guard: the browser's own
  // prompt, armed only while something is dirty anywhere. In-app navigation is covered by the
  // drafts surviving it, which is why leaving the window is the only moment work can be lost.
  const dirtyCount = Object.values(drafts).filter(isDirty).length;
  useEffect(() => {
    if (dirtyCount === 0) return;
    const warn = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [dirtyCount]);

  return {
    draft,
    dirtyCount,
    isDirty: draft ? isDirty(draft) : false,
    canSave: draft ? canSave(draft) : false,
    isLoading: !incomplete && policyQuery.isPending,
    loadError: policyQuery.error,
    edit: (patch: Partial<InstructionValues>) =>
      setDrafts((current) => editDraft(current, scope, patch)),
    discard: () => setDrafts((current) => discardDraft(current, scope)),
    save: () => {
      const current = drafts[key];
      if (current && canSave(current)) saveMutation.mutate(current.values);
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
