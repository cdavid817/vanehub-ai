import type {
  InstructionMergeMode,
  PersonalizationPolicy,
  PersonalizationPolicyRef,
} from "../../../types/personalization";

// The instruction draft engine is a pure module so its rules can be tested without mounting
// anything. Every rule here exists because of a way a settings page can lose a user's work: a
// refetch overwriting half-typed text, a failed save clearing the field it failed to save, and a
// conflict resolved by whichever response happened to land last.

export interface InstructionValues {
  aboutUser: string;
  styleRules: string;
  instructionMergeMode: InstructionMergeMode;
}

/** What the store held when it refused the save, kept beside the draft so the user can compare. */
export interface InstructionConflict {
  stored: InstructionValues;
  storedRevision: number;
  attemptedRevision: number;
}

export interface InstructionDraft {
  scope: PersonalizationPolicyRef;
  /** 0 means the layer has never been written -- distinct from a layer written to all-inherit. */
  baseRevision: number;
  baseline: InstructionValues;
  values: InstructionValues;
  conflict: InstructionConflict | null;
  saving: boolean;
  /** A non-conflict failure. The draft survives it; only the message is transient. */
  error: string | null;
}

export type InstructionDraftMap = Record<string, InstructionDraft>;

/**
 * Injective for any scope, including keys containing separators.
 *
 * Joining the parts with a delimiter would collide the moment a workspace key contained it, and
 * two scopes sharing a draft is the bug that puts one layer's half-typed text into another's.
 */
export function scopeKeyOf(scope: PersonalizationPolicyRef): string {
  return JSON.stringify([scope.scopeKind, scope.agentId ?? null, scope.workspaceKey ?? null]);
}

const INHERITED: InstructionValues = {
  aboutUser: "",
  styleRules: "",
  instructionMergeMode: "inherit",
};

function valuesOf(policy: PersonalizationPolicy | null): InstructionValues {
  if (!policy) return INHERITED;
  return {
    aboutUser: policy.aboutUser,
    styleRules: policy.styleRules,
    instructionMergeMode: policy.instructionMergeMode,
  };
}

export function draftFromPolicy(
  scope: PersonalizationPolicyRef,
  policy: PersonalizationPolicy | null,
): InstructionDraft {
  const values = valuesOf(policy);
  return {
    scope,
    baseRevision: policy?.revision ?? 0,
    baseline: values,
    values,
    conflict: null,
    saving: false,
    error: null,
  };
}

export function sameValues(left: InstructionValues, right: InstructionValues): boolean {
  return (
    left.aboutUser === right.aboutUser
    && left.styleRules === right.styleRules
    && left.instructionMergeMode === right.instructionMergeMode
  );
}

export function isDirty(draft: InstructionDraft): boolean {
  return !sameValues(draft.baseline, draft.values);
}

/** Refused while the store has moved underneath the draft: the user has to choose first. */
export function canSave(draft: InstructionDraft): boolean {
  return isDirty(draft) && !draft.saving && draft.conflict === null;
}

/**
 * Reconciles a refetch against whatever the user is editing.
 *
 * A clean draft follows the store. A dirty draft survives a refetch that changed nothing, and
 * becomes a conflict -- never a silent overwrite -- when the revision moved.
 */
export function mergePolicies(
  drafts: InstructionDraftMap,
  scopes: readonly { scope: PersonalizationPolicyRef; policy: PersonalizationPolicy | null }[],
): InstructionDraftMap {
  const next = { ...drafts };
  for (const { scope, policy } of scopes) {
    const key = scopeKeyOf(scope);
    const draft = next[key];
    if (!draft) {
      next[key] = draftFromPolicy(scope, policy);
      continue;
    }
    const storedRevision = policy?.revision ?? 0;
    if (!isDirty(draft)) {
      next[key] = draftFromPolicy(scope, policy);
      continue;
    }
    if (storedRevision === draft.baseRevision) continue;
    next[key] = {
      ...draft,
      conflict: {
        stored: valuesOf(policy),
        storedRevision,
        attemptedRevision: draft.baseRevision,
      },
    };
  }
  return next;
}

function update(
  drafts: InstructionDraftMap,
  scope: PersonalizationPolicyRef,
  project: (draft: InstructionDraft) => InstructionDraft,
): InstructionDraftMap {
  const key = scopeKeyOf(scope);
  const draft = drafts[key];
  if (!draft) return drafts;
  return { ...drafts, [key]: project(draft) };
}

export function editDraft(
  drafts: InstructionDraftMap,
  scope: PersonalizationPolicyRef,
  patch: Partial<InstructionValues>,
): InstructionDraftMap {
  // Editing clears the last error but not a conflict: the error described an attempt that is over,
  // while the conflict describes the store, which typing does not change.
  return update(drafts, scope, (draft) => ({
    ...draft,
    values: { ...draft.values, ...patch },
    error: null,
  }));
}

export function discardDraft(
  drafts: InstructionDraftMap,
  scope: PersonalizationPolicyRef,
): InstructionDraftMap {
  return update(drafts, scope, (draft) => ({
    ...draft,
    values: draft.baseline,
    conflict: null,
    error: null,
  }));
}

export function beginSave(
  drafts: InstructionDraftMap,
  scope: PersonalizationPolicyRef,
): InstructionDraftMap {
  return update(drafts, scope, (draft) => ({ ...draft, saving: true, error: null }));
}

export function saveSucceeded(
  drafts: InstructionDraftMap,
  scope: PersonalizationPolicyRef,
  policy: PersonalizationPolicy,
): InstructionDraftMap {
  return update(drafts, scope, () => draftFromPolicy(scope, policy));
}

/** The draft survives. A page that cleared the field on failure would discard the only copy. */
export function saveFailed(
  drafts: InstructionDraftMap,
  scope: PersonalizationPolicyRef,
  message: string,
): InstructionDraftMap {
  return update(drafts, scope, (draft) => ({ ...draft, saving: false, error: message }));
}

export function saveConflicted(
  drafts: InstructionDraftMap,
  scope: PersonalizationPolicyRef,
  stored: PersonalizationPolicy | null,
): InstructionDraftMap {
  return update(drafts, scope, (draft) => ({
    ...draft,
    saving: false,
    error: null,
    conflict: {
      stored: valuesOf(stored),
      storedRevision: stored?.revision ?? 0,
      attemptedRevision: draft.baseRevision,
    },
  }));
}

/**
 * Retarget the user's text at the revision that refused it, so the retry can land.
 *
 * This is the only path by which a draft overwrites a newer stored value, and it exists precisely
 * so that overwriting is something the user chose rather than something a race decided.
 */
export function keepMine(
  drafts: InstructionDraftMap,
  scope: PersonalizationPolicyRef,
): InstructionDraftMap {
  return update(drafts, scope, (draft) => {
    if (!draft.conflict) return draft;
    return {
      ...draft,
      baseRevision: draft.conflict.storedRevision,
      baseline: draft.conflict.stored,
      conflict: null,
    };
  });
}

export function takeTheirs(
  drafts: InstructionDraftMap,
  scope: PersonalizationPolicyRef,
): InstructionDraftMap {
  return update(drafts, scope, (draft) => {
    if (!draft.conflict) return draft;
    return {
      ...draft,
      baseRevision: draft.conflict.storedRevision,
      baseline: draft.conflict.stored,
      values: draft.conflict.stored,
      conflict: null,
    };
  });
}
