import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import type { AgentService } from "../../../services/agent-service";
import type {
  MemoryDetail,
  MemoryQueryStatus,
  UpdateMemoryInput,
} from "../../../types/personalization-memory";
import type { MemorySensitivity, MemoryType } from "../../../types/personalization";

/** The code the native command layer sends; the message is the whole contract across the wire. */
const CONFLICT_CODE = "personalization-revision-conflict";

export interface MemoryDraft {
  name: string;
  description: string;
  content: string;
  memoryType: Exclude<MemoryType, "untyped">;
  sensitivity: MemorySensitivity;
}

export function draftOf(record: MemoryDetail): MemoryDraft {
  return {
    name: record.name,
    description: record.description,
    content: record.content,
    // A migrated record can be `untyped`, which is not a value a caller may send back. Editing one
    // has to choose a real type, and seeding the editor with `user` makes that a choice the user
    // sees rather than a save refused after they wrote a paragraph.
    memoryType: record.memoryType === "untyped" ? "user" : record.memoryType,
    sensitivity: record.sensitivity,
  };
}

export function isDraftDirty(draft: MemoryDraft, record: MemoryDetail): boolean {
  const baseline = draftOf(record);
  return (Object.keys(baseline) as (keyof MemoryDraft)[]).some((key) => draft[key] !== baseline[key]);
}

export function memoryDetailQueryKey(memoryId: string) {
  return ["personalization", "memory", memoryId] as const;
}

/**
 * One memory, its edits, and the two lifecycle actions.
 *
 * Every write carries the revision the user was looking at. The delete does too: a delete without
 * one removes whatever is there now, so a stale panel destroys an edit its owner never saw.
 */
export function useMemoryDetail(service: AgentService, memoryId: string | null) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<MemoryDraft | null>(null);
  const [editing, setEditing] = useState(false);
  const [conflict, setConflict] = useState(false);
  const [failure, setFailure] = useState<"save" | "delete" | null>(null);

  const detailQuery = useQuery({
    queryKey: memoryDetailQueryKey(memoryId ?? ""),
    queryFn: () => service.getPersonalizationMemory(memoryId ?? ""),
    enabled: memoryId !== null,
  });
  const record = detailQuery.data ?? null;

  useEffect(() => {
    // A different memory is a different editor. Carrying a draft across would offer one record's
    // text as an edit to another.
    setDraft(null);
    setEditing(false);
    setConflict(false);
    setFailure(null);
  }, [memoryId]);

  function invalidateLists() {
    void queryClient.invalidateQueries({ queryKey: ["personalization", "memories"] });
    void queryClient.invalidateQueries({ queryKey: ["personalization", "overview"] });
  }

  const updateMutation = useMutation({
    mutationFn: (input: UpdateMemoryInput) => service.updatePersonalizationMemory(input),
    onSuccess: (updated) => {
      queryClient.setQueryData(memoryDetailQueryKey(updated.id), updated);
      invalidateLists();
      setDraft(null);
      setEditing(false);
      setConflict(false);
      setFailure(null);
    },
    onError: async (error: Error) => {
      if (!error.message.includes(CONFLICT_CODE)) {
        setFailure("save");
        return;
      }
      // Re-read rather than trust the numbers in the message: the panel is about to show what the
      // store holds, and the user decides against that, not against a revision number.
      setConflict(true);
      setFailure(null);
      await queryClient.invalidateQueries({ queryKey: memoryDetailQueryKey(memoryId ?? "") });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (input: { id: string; expectedRevision: number }) =>
      service.deletePersonalizationMemory(input.id, input.expectedRevision),
    onSuccess: () => {
      queryClient.removeQueries({ queryKey: memoryDetailQueryKey(memoryId ?? "") });
      invalidateLists();
    },
    onError: (error: Error) => {
      const raced = error.message.includes(CONFLICT_CODE);
      setConflict(raced);
      setFailure(raced ? null : "delete");
    },
  });

  function submit(patch: Partial<UpdateMemoryInput>) {
    if (!record) return;
    updateMutation.mutate({ id: record.id, expectedRevision: record.revision, ...patch });
  }

  return {
    record,
    draft: draft ?? (record ? draftOf(record) : null),
    editing,
    conflict,
    failure,
    isLoading: memoryId !== null && detailQuery.isPending,
    loadError: detailQuery.error,
    isSaving: updateMutation.isPending || deleteMutation.isPending,
    isDirty: draft !== null && record !== null && isDraftDirty(draft, record),
    beginEdit: () => {
      if (record) setDraft(draftOf(record));
      setEditing(true);
      setFailure(null);
    },
    edit: (patch: Partial<MemoryDraft>) =>
      setDraft((current) => (current ? { ...current, ...patch } : current)),
    cancelEdit: () => {
      setDraft(null);
      setEditing(false);
      setFailure(null);
    },
    save: () => {
      if (draft) submit(draft);
    },
    /** Archive and reactivate are the same write with a different status, so they cannot diverge. */
    setStatus: (status: MemoryQueryStatus) => submit({ status }),
    remove: () => {
      if (record) deleteMutation.mutate({ id: record.id, expectedRevision: record.revision });
    },
    deleted: deleteMutation.isSuccess,
    /** Drops the local edit and shows what the store has, which is what resolves a conflict here. */
    reload: () => {
      setDraft(null);
      setEditing(false);
      setConflict(false);
      void queryClient.invalidateQueries({ queryKey: memoryDetailQueryKey(memoryId ?? "") });
    },
  };
}
