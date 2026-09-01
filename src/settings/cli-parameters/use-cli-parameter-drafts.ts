import { useCallback, useEffect, useMemo, useState } from "react";
import type { ManagedCliAgentId } from "../../types/agent";
import type {
  CliParameterDefinition,
  CliParameterSelection,
  CliParameterSelections,
} from "../../types/cli-parameter";
import type { CliParameterProfile } from "../../types/cli-parameter-profile";
import {
  canSave,
  dirtyIds,
  discardDraft,
  enterCustomMode,
  isDirty,
  markSaved,
  mergeProfiles,
  reloadFromServer,
  restoreInherited,
  setCustomInput,
  setSelection,
  type CliParameterConflict,
  type CliParameterDraft,
  type CliParameterDraftMap,
} from "./draft-state";

const emptyDraft: CliParameterDraft = {
  baselineRevision: 0,
  baselineCatalogVersion: "",
  baselineSelections: {},
  selections: {},
  customInputs: {},
  customMode: [],
  invalidIds: [],
  conflict: "none",
};

export interface CliParameterDraftsApi {
  draftFor(agentId: ManagedCliAgentId): CliParameterDraft;
  selectionsFor(agentId: ManagedCliAgentId): CliParameterSelections;
  dirtyIdsFor(agentId: ManagedCliAgentId): string[];
  isDirtyFor(agentId: ManagedCliAgentId): boolean;
  canSaveFor(agentId: ManagedCliAgentId): boolean;
  conflictFor(agentId: ManagedCliAgentId): CliParameterConflict;
  isCustomMode(agentId: ManagedCliAgentId, parameterId: string): boolean;
  customInputFor(agentId: ManagedCliAgentId, parameterId: string): string;
  isInvalid(agentId: ManagedCliAgentId, parameterId: string): boolean;
  totalDirtyCount: number;
  select(agentId: ManagedCliAgentId, parameterId: string, selection: CliParameterSelection): void;
  openCustom(agentId: ManagedCliAgentId, parameterId: string, seed: string): void;
  typeCustom(agentId: ManagedCliAgentId, parameterId: string, text: string): void;
  discard(agentId: ManagedCliAgentId): void;
  inheritAll(agentId: ManagedCliAgentId, definitions: readonly CliParameterDefinition[]): void;
  accept(profile: CliParameterProfile): void;
  reload(profile: CliParameterProfile): void;
}

/**
 * Holds one draft per managed CLI, so switching CLIs in the rail never loses an edit, and merges
 * server refetches without overwriting work in progress.
 */
export function useCliParameterDrafts(
  profiles: readonly CliParameterProfile[],
): CliParameterDraftsApi {
  const [drafts, setDrafts] = useState<CliParameterDraftMap>({});

  useEffect(() => {
    if (profiles.length === 0) return;
    setDrafts((current) => mergeProfiles(current, profiles));
  }, [profiles]);

  const draftFor = useCallback(
    (agentId: ManagedCliAgentId) => drafts[agentId] ?? emptyDraft,
    [drafts],
  );

  const totalDirtyCount = useMemo(
    () =>
      Object.values(drafts).reduce(
        (total, draft) => total + (draft ? dirtyIds(draft).length : 0),
        0,
      ),
    [drafts],
  );

  // Closing the whole window/tab is not routed through React at all, so the shell's own
  // navigation guard (task 12.12, `settings-shell.tsx`) cannot intercept it -- this is the only
  // guard for that specific departure: the browser's own prompt, armed only while something is
  // actually dirty. Every *in-app* departure (an inter-page switch, or leaving Settings for the
  // workbench) is covered by the shell's guard instead, which -- unlike this blunt yes/no prompt
  // -- can actually offer Save.
  useEffect(() => {
    if (totalDirtyCount === 0) return;
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault();
    };
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [totalDirtyCount]);

  return {
    draftFor,
    totalDirtyCount,
    selectionsFor: (agentId) => draftFor(agentId).selections,
    dirtyIdsFor: (agentId) => dirtyIds(draftFor(agentId)),
    isDirtyFor: (agentId) => isDirty(draftFor(agentId)),
    canSaveFor: (agentId) => canSave(draftFor(agentId)),
    conflictFor: (agentId) => draftFor(agentId).conflict,
    isCustomMode: (agentId, parameterId) => draftFor(agentId).customMode.includes(parameterId),
    customInputFor: (agentId, parameterId) => draftFor(agentId).customInputs[parameterId] ?? "",
    isInvalid: (agentId, parameterId) => draftFor(agentId).invalidIds.includes(parameterId),
    select: (agentId, parameterId, selection) =>
      setDrafts((current) => setSelection(current, agentId, parameterId, selection)),
    openCustom: (agentId, parameterId, seed) =>
      setDrafts((current) => enterCustomMode(current, agentId, parameterId, seed)),
    typeCustom: (agentId, parameterId, text) =>
      setDrafts((current) => setCustomInput(current, agentId, parameterId, text)),
    discard: (agentId) => setDrafts((current) => discardDraft(current, agentId)),
    inheritAll: (agentId, definitions) =>
      setDrafts((current) => restoreInherited(current, agentId, definitions)),
    accept: (profile) => setDrafts((current) => markSaved(current, profile)),
    reload: (profile) => setDrafts((current) => reloadFromServer(current, profile)),
  };
}
