import type { ManagedCliAgentId } from "../../types/agent";
import type {
  CliParameterDefinition,
  CliParameterSelection,
  CliParameterSelections,
} from "../../types/cli-parameter";
import type { CliParameterProfile } from "../../types/cli-parameter-profile";

// The draft engine is a pure reducer so its rules can be tested without mounting anything. Every
// rule here exists because of a way the previous page got it wrong: transient custom text leaking
// between CLIs, an empty custom box silently becoming a saved empty string, and a refetch throwing
// away edits the user had not saved yet.

export type CliParameterConflict = "none" | "revision" | "catalog";

export interface CliParameterDraft {
  baselineRevision: number;
  baselineCatalogVersion: string;
  baselineSelections: CliParameterSelections;
  selections: CliParameterSelections;
  /** Keyed by parameter id, and the whole draft is keyed by agent id, so `claude-code:model`'s
   * half-typed text cannot appear in `codex-cli:model`. */
  customInputs: Record<string, string>;
  customMode: string[];
  invalidIds: string[];
  conflict: CliParameterConflict;
}

export type CliParameterDraftMap = Partial<Record<ManagedCliAgentId, CliParameterDraft>>;

function fromProfile(profile: CliParameterProfile): CliParameterDraft {
  return {
    baselineRevision: profile.revision,
    baselineCatalogVersion: profile.catalogVersion,
    baselineSelections: profile.selections,
    selections: profile.selections,
    customInputs: {},
    customMode: [],
    invalidIds: [],
    conflict: "none",
  };
}

export function sameSelection(left: CliParameterSelection, right: CliParameterSelection): boolean {
  if (left.state !== right.state) return false;
  if (left.state === "inherit" || right.state === "inherit") return true;
  if (Array.isArray(left.value) || Array.isArray(right.value)) {
    if (!Array.isArray(left.value) || !Array.isArray(right.value)) return false;
    const other = right.value;
    return left.value.length === other.length && left.value.every((entry, index) => entry === other[index]);
  }
  return left.value === right.value;
}

export function dirtyIds(draft: CliParameterDraft): string[] {
  const ids = new Set([...Object.keys(draft.baselineSelections), ...Object.keys(draft.selections)]);
  return [...ids].filter((id) => {
    const baseline = draft.baselineSelections[id] ?? { state: "inherit" as const };
    const current = draft.selections[id] ?? { state: "inherit" as const };
    return !sameSelection(baseline, current);
  });
}

export function isDirty(draft: CliParameterDraft): boolean {
  return dirtyIds(draft).length > 0;
}

/** Save is refused while a field is locally invalid or the server moved underneath the draft. */
export function canSave(draft: CliParameterDraft): boolean {
  return isDirty(draft) && draft.invalidIds.length === 0 && draft.conflict === "none";
}

/**
 * Reconciles a refetch against whatever the user is editing.
 *
 * An untouched draft simply follows the server. A dirty draft survives a refetch that changed
 * nothing, and becomes a conflict — not a silent overwrite — when the revision or the catalog moved.
 */
export function mergeProfiles(
  current: CliParameterDraftMap,
  profiles: readonly CliParameterProfile[],
): CliParameterDraftMap {
  const next: CliParameterDraftMap = { ...current };
  for (const profile of profiles) {
    const draft = next[profile.agentId];
    if (!draft) {
      next[profile.agentId] = fromProfile(profile);
      continue;
    }
    if (!isDirty(draft)) {
      next[profile.agentId] = { ...fromProfile(profile), customInputs: draft.customInputs, customMode: draft.customMode };
      continue;
    }
    const catalogMoved = draft.baselineCatalogVersion !== profile.catalogVersion;
    const revisionMoved = draft.baselineRevision !== profile.revision;
    if (!catalogMoved && !revisionMoved) continue;
    next[profile.agentId] = { ...draft, conflict: catalogMoved ? "catalog" : "revision" };
  }
  return next;
}

function update(
  map: CliParameterDraftMap,
  agentId: ManagedCliAgentId,
  project: (draft: CliParameterDraft) => CliParameterDraft,
): CliParameterDraftMap {
  const draft = map[agentId];
  if (!draft) return map;
  return { ...map, [agentId]: project(draft) };
}

function withoutInvalid(draft: CliParameterDraft, parameterId: string): string[] {
  return draft.invalidIds.filter((id) => id !== parameterId);
}

export function setSelection(
  map: CliParameterDraftMap,
  agentId: ManagedCliAgentId,
  parameterId: string,
  selection: CliParameterSelection,
): CliParameterDraftMap {
  return update(map, agentId, (draft) => ({
    ...draft,
    selections: { ...draft.selections, [parameterId]: selection },
    customMode: draft.customMode.filter((id) => id !== parameterId),
    invalidIds: withoutInvalid(draft, parameterId),
  }));
}

/**
 * Switching a control to Custom changes the editor mode and nothing else. It never writes a value,
 * so a user who opens Custom and changes their mind has not modified the profile. The field is
 * invalid until it holds text, which is what keeps an empty box from being saved as an empty value.
 */
export function enterCustomMode(
  map: CliParameterDraftMap,
  agentId: ManagedCliAgentId,
  parameterId: string,
  seed: string,
): CliParameterDraftMap {
  return update(map, agentId, (draft) => ({
    ...draft,
    customInputs: { ...draft.customInputs, [parameterId]: seed },
    customMode: draft.customMode.includes(parameterId)
      ? draft.customMode
      : [...draft.customMode, parameterId],
    invalidIds:
      seed.trim().length > 0
        ? withoutInvalid(draft, parameterId)
        : [...withoutInvalid(draft, parameterId), parameterId],
  }));
}

export function setCustomInput(
  map: CliParameterDraftMap,
  agentId: ManagedCliAgentId,
  parameterId: string,
  text: string,
): CliParameterDraftMap {
  return update(map, agentId, (draft) => {
    const trimmed = text.trim();
    return {
      ...draft,
      customInputs: { ...draft.customInputs, [parameterId]: text },
      customMode: draft.customMode.includes(parameterId)
        ? draft.customMode
        : [...draft.customMode, parameterId],
      // An empty box is a local validation state, not a transport value: the previous selection
      // stays exactly as it was until valid text replaces it.
      selections:
        trimmed.length > 0
          ? { ...draft.selections, [parameterId]: { state: "value", value: trimmed } }
          : draft.selections,
      invalidIds:
        trimmed.length > 0
          ? withoutInvalid(draft, parameterId)
          : [...withoutInvalid(draft, parameterId), parameterId],
    };
  });
}

export function discardDraft(
  map: CliParameterDraftMap,
  agentId: ManagedCliAgentId,
): CliParameterDraftMap {
  return update(map, agentId, (draft) => ({
    ...draft,
    selections: draft.baselineSelections,
    customInputs: {},
    customMode: [],
    invalidIds: [],
    conflict: "none",
  }));
}

export function restoreInherited(
  map: CliParameterDraftMap,
  agentId: ManagedCliAgentId,
  definitions: readonly CliParameterDefinition[],
): CliParameterDraftMap {
  return update(map, agentId, (draft) => ({
    ...draft,
    selections: Object.fromEntries(
      definitions.map((definition) => [definition.id, { state: "inherit" as const }]),
    ),
    customInputs: {},
    customMode: [],
    invalidIds: [],
  }));
}

/** After a successful write the server's profile becomes the new baseline, which also clears the
 * dirty state without a second refetch. */
export function markSaved(
  map: CliParameterDraftMap,
  profile: CliParameterProfile,
): CliParameterDraftMap {
  return { ...map, [profile.agentId]: fromProfile(profile) };
}

/** Reload after a conflict: the server wins and the draft is gone, which is what the reload action
 * promises. */
export function reloadFromServer(
  map: CliParameterDraftMap,
  profile: CliParameterProfile,
): CliParameterDraftMap {
  return markSaved(map, profile);
}
